use super::*;

pub(super) fn project_from_instrument(
    instrument: &Instrument,
    bpm: u16,
    scale_index: usize,
) -> Result<Project, String> {
    validate_composition_labels(&instrument.root)?;
    let mut composition_ids = HashMap::new();
    let mut composition_ports = HashMap::new();
    collect_composition_metadata(
        &instrument.root,
        &mut composition_ids,
        &mut composition_ports,
    );
    Ok(Project {
        bpm: bpm as f32,
        bars: 1.0,
        scale_idx: scale_index,
        modules: project_modules_from_surface(
            &instrument.root,
            &composition_ids,
            &composition_ports,
        )?,
        track: Some(instrument.track_text.clone()),
        compositions: project_compositions_from_surface(
            &instrument.root,
            &composition_ids,
            &composition_ports,
        )?,
    })
}

fn validate_composition_labels(surface: &PatchSurface) -> Result<(), String> {
    for module in &surface.modules {
        let Some(composition) = module.composition_surface() else {
            continue;
        };
        let mut input_labels = Vec::new();
        let mut output_labels = Vec::new();
        for port in composition.modules.iter().filter(|module| {
            matches!(
                module.kind(),
                ModuleKind::CompositionInput | ModuleKind::CompositionOutput
            )
        }) {
            let (label, labels) = match &port.body {
                ModuleBody::CompositionInput { label, .. } => (label.trim(), &mut input_labels),
                ModuleBody::CompositionOutput { label, .. } => (label.trim(), &mut output_labels),
                _ => unreachable!(),
            };
            if label.is_empty() || labels.iter().any(|candidate| *candidate == label) {
                return Err(format!("invalid composition port label {label:?}"));
            }
            labels.push(label);
        }
        validate_composition_labels(composition)?;
    }
    Ok(())
}

fn collect_composition_metadata(
    surface: &PatchSurface,
    ids: &mut HashMap<ModuleId, u32>,
    ports: &mut HashMap<ModuleId, (u8, u8)>,
) {
    for module in &surface.modules {
        if module.kind() == ModuleKind::Composition
            && let Some(composition) = module.composition_surface()
        {
            ids.insert(module.id, module.id.value());
            ports.insert(module.id, composition_port_counts(composition));
            collect_composition_metadata(composition, ids, ports);
        }
    }
}

fn project_compositions_from_surface(
    surface: &PatchSurface,
    ids: &HashMap<ModuleId, u32>,
    ports: &HashMap<ModuleId, (u8, u8)>,
) -> Result<Vec<ProjectCompositionDef>, String> {
    let mut compositions = Vec::new();
    for module in &surface.modules {
        if module.kind() != ModuleKind::Composition {
            continue;
        }
        let Some(surface) = module.composition_surface() else {
            continue;
        };
        let ModuleBody::Composition { name, .. } = &module.body else {
            unreachable!();
        };
        compositions.push(ProjectCompositionDef {
            id: module.id.value(),
            name: name.clone(),
            color: (0, 0, 0),
            modules: project_modules_from_surface(surface, ids, ports)?,
        });
        compositions.extend(project_compositions_from_surface(surface, ids, ports)?);
    }
    Ok(compositions)
}

fn composition_port_counts(surface: &PatchSurface) -> (u8, u8) {
    let inputs = surface
        .modules
        .iter()
        .filter(|module| module.kind() == ModuleKind::CompositionInput)
        .count()
        .min(u8::MAX as usize) as u8;
    let outputs = surface
        .modules
        .iter()
        .filter(|module| module.kind() == ModuleKind::CompositionOutput)
        .count()
        .min(u8::MAX as usize) as u8;
    (inputs, outputs)
}

fn project_modules_from_surface(
    surface: &PatchSurface,
    composition_ids: &HashMap<ModuleId, u32>,
    composition_ports: &HashMap<ModuleId, (u8, u8)>,
) -> Result<Vec<ProjectModuleDef>, String> {
    surface
        .modules
        .iter()
        .map(|module| {
            Ok(ProjectModuleDef {
                id: module.id,
                kind: project_kind(module, &surface.modules, composition_ids)?,
                x: module.position.x,
                y: module.position.y,
                orientation: module.orientation,
                params: project_params(module, composition_ports)?,
            })
        })
        .collect()
}

fn project_kind(
    module: &Module,
    modules: &[Module],
    composition_ids: &HashMap<ModuleId, u32>,
) -> Result<ProjectModuleKind, String> {
    Ok(match module.kind() {
        ModuleKind::Primitive => ProjectModuleKind::Standard(ProjectStandardModule::Primitive),
        ModuleKind::Constant
        | ModuleKind::Absolute
        | ModuleKind::Sine
        | ModuleKind::Tanh
        | ModuleKind::Atan
        | ModuleKind::Exp
        | ModuleKind::Sign
        | ModuleKind::Subtract
        | ModuleKind::Divide
        | ModuleKind::Power
        | ModuleKind::Remainder
        | ModuleKind::Minimum
        | ModuleKind::Maximum
        | ModuleKind::Multiply
        | ModuleKind::Add
        | ModuleKind::GreaterThan
        | ModuleKind::LessThan
        | ModuleKind::Equal
        | ModuleKind::Switch
        | ModuleKind::Filter => unreachable!(),
        ModuleKind::Damp | ModuleKind::Slew => unreachable!(),
        ModuleKind::TurnRightDown => ProjectModuleKind::Routing(ProjectRoutingModule::TurnRD),
        ModuleKind::TurnDownRight => ProjectModuleKind::Routing(ProjectRoutingModule::TurnDR),
        ModuleKind::LeftSplit => ProjectModuleKind::Routing(ProjectRoutingModule::LSplit),
        ModuleKind::TopSplit => ProjectModuleKind::Routing(ProjectRoutingModule::TSplit),
        ModuleKind::RightJoin => ProjectModuleKind::Routing(ProjectRoutingModule::RJoin),
        ModuleKind::DownJoin => ProjectModuleKind::Routing(ProjectRoutingModule::DJoin),
        ModuleKind::CompositionInput => {
            ProjectModuleKind::Composition(ProjectCompositionModule::Input)
        }
        ModuleKind::CompositionOutput => {
            ProjectModuleKind::Composition(ProjectCompositionModule::Output)
        }
        ModuleKind::Composition => {
            let id = composition_ids
                .get(&module.id)
                .ok_or_else(|| format!("missing composition surface {}", module.id.value()))?;
            ProjectModuleKind::Composition(ProjectCompositionModule::Composition(
                ProjectCompositionId::new(*id),
            ))
        }
        ModuleKind::DelayTap => ProjectModuleKind::Standard(ProjectStandardModule::DelayTap(
            delay_source(module, modules)?,
        )),
        ModuleKind::Freq => ProjectModuleKind::Standard(ProjectStandardModule::Freq),
        ModuleKind::Gate => ProjectModuleKind::Standard(ProjectStandardModule::Gate),
        ModuleKind::Degree => ProjectModuleKind::Standard(ProjectStandardModule::Degree),
        ModuleKind::Phase => ProjectModuleKind::Standard(ProjectStandardModule::Phase),
        ModuleKind::Noise => ProjectModuleKind::Standard(ProjectStandardModule::Noise),
        ModuleKind::Rise => ProjectModuleKind::Standard(ProjectStandardModule::Rise),
        ModuleKind::Fall => ProjectModuleKind::Standard(ProjectStandardModule::Fall),
        ModuleKind::Ramp => ProjectModuleKind::Standard(ProjectStandardModule::Ramp),
        ModuleKind::Envelope => ProjectModuleKind::Standard(ProjectStandardModule::Envelope),
        ModuleKind::Comb => ProjectModuleKind::Standard(ProjectStandardModule::Comb),
        ModuleKind::Allpass => ProjectModuleKind::Standard(ProjectStandardModule::Allpass),
        ModuleKind::Delay => ProjectModuleKind::Standard(ProjectStandardModule::Delay),
        ModuleKind::Random => ProjectModuleKind::Standard(ProjectStandardModule::Rng),
        ModuleKind::Sample => ProjectModuleKind::Standard(ProjectStandardModule::Sample),
        ModuleKind::Probe => ProjectModuleKind::Standard(ProjectStandardModule::Probe),
        ModuleKind::Output => ProjectModuleKind::Standard(ProjectStandardModule::Output),
    })
}

fn delay_source(module: &Module, modules: &[Module]) -> Result<ModuleId, String> {
    let ModuleBody::DelayTap { source, .. } = &module.body else {
        return Err(format!("module {} is not a delay tap", module.id.value()));
    };
    let selected = source
        .selected
        .ok_or_else(|| format!("missing delay source {}", module.id.value()))?;
    modules
        .iter()
        .any(|module| module.id == selected && module.kind() == ModuleKind::Delay)
        .then_some(selected)
        .ok_or_else(|| format!("missing delay source {}", module.id.value()))
}

fn project_params(
    module: &Module,
    composition_ports: &HashMap<ModuleId, (u8, u8)>,
) -> Result<ProjectModuleParams, String> {
    Ok(match &module.body {
        ModuleBody::Primitive(module) => ProjectModuleParams::Primitive {
            source: brainwash::persist::module_to_string(module)
                .map_err(|_| format!("invalid primitive {}", graph_node_label(module)))?,
        },
        ModuleBody::Freq
        | ModuleBody::Gate
        | ModuleBody::Degree
        | ModuleBody::TurnRightDown
        | ModuleBody::TurnDownRight
        | ModuleBody::LeftSplit
        | ModuleBody::TopSplit
        | ModuleBody::RightJoin
        | ModuleBody::DownJoin => ProjectModuleParams::None,
        ModuleBody::CompositionInput { label, kind, value } => {
            ProjectModuleParams::CompositionInput {
                label: label.clone(),
                kind: *kind,
                value: project_float(*value),
                connected: value.connected,
            }
        }
        ModuleBody::Phase { frequency } => ProjectModuleParams::Phase {
            frequency: project_float(*frequency),
            connected: connected_mask(&[(0, frequency.connected)]),
        },
        ModuleBody::Rise { gate, time } => ProjectModuleParams::Rise {
            gate: project_float(*gate),
            time: project_time(*time)?,
            connected: connected_mask(&[(0, gate.connected), (1, time.connected)]),
        },
        ModuleBody::Fall { gate, time } => ProjectModuleParams::Fall {
            gate: project_float(*gate),
            time: project_time(*time)?,
            connected: connected_mask(&[(0, gate.connected), (1, time.connected)]),
        },
        ModuleBody::Ramp { value, time } => ProjectModuleParams::Ramp {
            value: project_float(*value),
            time: project_time(*time)?,
            connected: connected_mask(&[(0, value.connected), (1, time.connected)]),
        },
        ModuleBody::Envelope { phase, points } => ProjectModuleParams::Envelope {
            phase: project_float(*phase),
            points: points.iter().map(project_env_point).collect(),
            connected: connected_mask(&[(0, phase.connected)]),
        },
        ModuleBody::Comb {
            input,
            time,
            feedback,
            damp,
        } => ProjectModuleParams::Comb {
            input: project_float(*input),
            time: project_time(*time)?,
            feedback: project_float(*feedback),
            damp: project_float(*damp),
            connected: connected_mask(&[
                (0, input.connected),
                (1, time.connected),
                (2, feedback.connected),
                (3, damp.connected),
            ]),
        },
        ModuleBody::Allpass {
            input,
            time,
            feedback,
        } => ProjectModuleParams::Allpass {
            input: project_float(*input),
            time: project_time(*time)?,
            feedback: project_float(*feedback),
            connected: connected_mask(&[
                (0, input.connected),
                (1, time.connected),
                (2, feedback.connected),
            ]),
        },
        ModuleBody::Delay {
            input,
            time,
            feedback,
        } => ProjectModuleParams::Delay {
            input: project_float(*input),
            time: project_time(*time)?,
            feedback: project_float(*feedback),
            connected: connected_mask(&[
                (0, feedback.connected),
                (1, time.connected),
                (2, input.connected),
            ]),
        },
        ModuleBody::DelayTap { gain, .. } => ProjectModuleParams::DelayTap {
            gain: project_float(*gain),
        },
        ModuleBody::Random { gate } => ProjectModuleParams::Random {
            gate: project_float(*gate),
            connected: connected_mask(&[(0, gate.connected)]),
        },
        ModuleBody::Sample {
            file_name,
            samples,
            position,
            ..
        } => ProjectModuleParams::Sample {
            file_idx: 0,
            file_name: file_name.clone(),
            samples: Arc::new(samples.iter().map(|sample| sample.value()).collect()),
            position: project_float(*position),
            connected: connected_mask(&[(1, position.connected)]),
        },
        ModuleBody::Probe { input } => ProjectModuleParams::Probe {
            input: project_float(*input),
            connected: connected_mask(&[(0, input.connected)]),
        },
        ModuleBody::Output { input, gain } => ProjectModuleParams::Output {
            input: project_float(*input),
            gain: project_float(*gain),
            connected: connected_mask(&[(0, input.connected), (1, gain.connected)]),
        },
        ModuleBody::CompositionOutput { label, input } => ProjectModuleParams::CompositionOutput {
            label: label.clone(),
            input: project_float(*input),
            connected: input.connected,
        },
        ModuleBody::Composition { .. } => {
            let (inputs, outputs) = composition_ports
                .get(&module.id)
                .copied()
                .ok_or_else(|| format!("missing composition ports {}", module.id.value()))?;
            ProjectModuleParams::Composition {
                inputs,
                outputs,
                color: (0, 0, 0),
            }
        }
    })
}

fn project_float(param: FloatParam) -> f32 {
    param.value as f32 / 100.0
}

fn project_time(param: TimeParam) -> Result<ProjectTimeValue, String> {
    let mut value = ProjectTimeValue {
        unit: ProjectTimeUnit::Seconds,
        seconds: 1.0,
        samples: 1.0,
        bar_num: 1,
        bar_denom: 16,
        hz: 1.0,
    };
    match param.unit {
        TimeUnit::Seconds => {
            value.unit = ProjectTimeUnit::Seconds;
            value.seconds = param
                .exact_seconds
                .map(f32::from_bits)
                .unwrap_or(param.value as f32 / 100.0);
        }
        TimeUnit::Samples => {
            value.unit = ProjectTimeUnit::Samples;
            value.samples = param.value as f32;
        }
        TimeUnit::Bars => {
            let bar_num = u8::try_from(param.value)
                .map_err(|_| format!("bar time out of range {}", param.value))?;
            let bar_denom = u8::try_from(param.denominator)
                .map_err(|_| format!("bar time denominator out of range {}", param.denominator))?;
            value.unit = ProjectTimeUnit::Bars;
            value.bar_num = bar_num;
            value.bar_denom = bar_denom.max(1);
        }
        TimeUnit::Hertz => {
            value.unit = ProjectTimeUnit::Hz;
            value.hz = param.value as f32;
        }
    }
    Ok(value)
}

fn project_env_point(point: &EnvPoint) -> brainwash_grid::project::EnvPoint {
    brainwash_grid::project::EnvPoint {
        time: point.time as f32 / 100.0,
        value: point.value as f32 / 100.0,
        curve: point.curve,
    }
}

fn connected_mask(rows: &[(usize, bool)]) -> u8 {
    rows.iter().fold(0, |mask, (index, connected)| {
        if *connected {
            mask | (1 << index)
        } else {
            mask
        }
    })
}

pub(super) fn instrument_surface_from_project(project: &Project) -> Result<PatchSurface, String> {
    let compositions = project
        .compositions
        .iter()
        .map(|composition| (composition.id, composition))
        .collect::<HashMap<_, _>>();
    surface_from_project_modules(&project.modules, &compositions)
}

fn surface_from_project_modules(
    definitions: &[ProjectModuleDef],
    compositions: &HashMap<u32, &ProjectCompositionDef>,
) -> Result<PatchSurface, String> {
    let mut modules = definitions
        .iter()
        .map(|definition| {
            let (mut module, composition_id) = module_from_project(definition)?;
            if let Some(composition_id) = composition_id {
                let definition = compositions
                    .get(&composition_id)
                    .ok_or_else(|| format!("missing composition {}", composition_id))?;
                module.body = ModuleBody::Composition {
                    name: definition.name.clone(),
                    surface: surface_from_project_modules(&definition.modules, compositions)?,
                };
            }
            Ok(module)
        })
        .collect::<Result<Vec<_>, String>>()?;
    for (definition, module) in definitions.iter().zip(&mut modules) {
        let ProjectModuleKind::Standard(ProjectStandardModule::DelayTap(source_id)) =
            definition.kind
        else {
            continue;
        };
        let options = definitions
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.kind,
                    ProjectModuleKind::Standard(ProjectStandardModule::Delay)
                )
            })
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        let selected = source_id;
        if !options.contains(&selected) {
            return Err(format!("missing delay source {}", source_id.value()));
        }
        let ModuleBody::DelayTap { source, .. } = &mut module.body else {
            return Err(format!(
                "module {} is not a delay tap",
                definition.id.value()
            ));
        };
        source.selected = Some(selected);
        source.options = options;
    }
    Ok(PatchSurface {
        cursor: GridPos::new(0, 0),
        modules,
    })
}

fn module_from_project(definition: &ProjectModuleDef) -> Result<(Module, Option<u32>), String> {
    let (kind, composition_id) = kind_from_project(definition.kind);
    let mut module = Module {
        id: definition.id,
        position: GridPos::new(definition.x, definition.y),
        orientation: definition.orientation,
        body: kind.default_body(),
        disabled: false,
    };
    if let ProjectModuleParams::Primitive { source } = &definition.params {
        module.body = ModuleBody::Primitive(
            brainwash::persist::module_from_str(source)
                .map_err(|_| format!("invalid primitive {}", definition.id.value()))?,
        );
    }
    apply_project_params(&mut module, &definition.params);
    Ok((module, composition_id))
}

fn kind_from_project(kind: ProjectModuleKind) -> (ModuleKind, Option<u32>) {
    match kind {
        ProjectModuleKind::Routing(routing) => (
            match routing {
                ProjectRoutingModule::LSplit => ModuleKind::LeftSplit,
                ProjectRoutingModule::TSplit => ModuleKind::TopSplit,
                ProjectRoutingModule::RJoin => ModuleKind::RightJoin,
                ProjectRoutingModule::DJoin => ModuleKind::DownJoin,
                ProjectRoutingModule::TurnRD => ModuleKind::TurnRightDown,
                ProjectRoutingModule::TurnDR => ModuleKind::TurnDownRight,
            },
            None,
        ),
        ProjectModuleKind::Composition(composition) => match composition {
            ProjectCompositionModule::Input => (ModuleKind::CompositionInput, None),
            ProjectCompositionModule::Output => (ModuleKind::CompositionOutput, None),
            ProjectCompositionModule::Composition(id) => {
                (ModuleKind::Composition, Some(id.value()))
            }
        },
        ProjectModuleKind::Standard(standard) => (
            match standard {
                ProjectStandardModule::Primitive => ModuleKind::Primitive,
                ProjectStandardModule::Freq => ModuleKind::Freq,
                ProjectStandardModule::Gate => ModuleKind::Gate,
                ProjectStandardModule::Degree => ModuleKind::Degree,
                ProjectStandardModule::Phase => ModuleKind::Phase,
                ProjectStandardModule::Noise => ModuleKind::Noise,
                ProjectStandardModule::Rise => ModuleKind::Rise,
                ProjectStandardModule::Fall => ModuleKind::Fall,
                ProjectStandardModule::Ramp => ModuleKind::Ramp,
                ProjectStandardModule::Envelope => ModuleKind::Envelope,
                ProjectStandardModule::Comb => ModuleKind::Comb,
                ProjectStandardModule::Allpass => ModuleKind::Allpass,
                ProjectStandardModule::Delay => ModuleKind::Delay,
                ProjectStandardModule::DelayTap(_) => ModuleKind::DelayTap,
                ProjectStandardModule::Rng => ModuleKind::Random,
                ProjectStandardModule::Sample => ModuleKind::Sample,
                ProjectStandardModule::Probe => ModuleKind::Probe,
                ProjectStandardModule::Output => ModuleKind::Output,
            },
            None,
        ),
    }
}

fn apply_project_params(module: &mut Module, params: &ProjectModuleParams) {
    let mut parameters = module.body.parameters();
    match params {
        ProjectModuleParams::None
        | ProjectModuleParams::Primitive { .. }
        | ProjectModuleParams::Composition { .. } => {}
        ProjectModuleParams::CompositionInput {
            label,
            kind,
            value,
            connected,
        } => {
            if let ModuleBody::CompositionInput { kind: target, .. } = &mut module.body {
                *target = *kind;
            }
            if let Some(parameter) = parameters.get_mut(0) {
                parameter.value = ParameterValue::Text(label.clone());
            }
            set_float(&mut parameters, 1, *value);
            if let Some(parameter) = parameters.get_mut(1) {
                parameter.connected = *connected;
            }
        }
        ProjectModuleParams::CompositionOutput {
            label,
            input,
            connected,
        } => {
            if let Some(parameter) = parameters.get_mut(0) {
                parameter.value = ParameterValue::Text(label.clone());
            }
            set_float(&mut parameters, 1, *input);
            if let Some(parameter) = parameters.get_mut(1) {
                parameter.connected = *connected;
            }
        }
        ProjectModuleParams::Phase {
            frequency,
            connected,
        } => {
            set_float(&mut parameters, 0, *frequency);
            if let Some(parameter) = parameters.get_mut(0) {
                parameter.connected = connected & 1 != 0;
            }
        }
        ProjectModuleParams::Rise {
            gate,
            time,
            connected,
        }
        | ProjectModuleParams::Fall {
            gate,
            time,
            connected,
        } => {
            set_float(&mut parameters, 0, *gate);
            set_time(&mut parameters, 1, *time);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Ramp {
            value,
            time,
            connected,
        } => {
            set_float(&mut parameters, 0, *value);
            set_time(&mut parameters, 1, *time);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Envelope {
            phase,
            points,
            connected,
        } => {
            set_float(&mut parameters, 0, *phase);
            if let Some(target) = module.body.env_points_mut() {
                *target = points.iter().map(env_point_from_project).collect();
            }
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Comb {
            input,
            time,
            feedback,
            damp,
            connected,
        } => {
            set_float(&mut parameters, 0, *input);
            set_time(&mut parameters, 1, *time);
            set_float(&mut parameters, 2, *feedback);
            set_float(&mut parameters, 3, *damp);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Allpass {
            input,
            time,
            feedback,
            connected,
        } => {
            set_float(&mut parameters, 0, *input);
            set_time(&mut parameters, 1, *time);
            set_float(&mut parameters, 2, *feedback);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Delay {
            input,
            time,
            feedback,
            connected,
        } => {
            set_float(&mut parameters, 2, *input);
            set_time(&mut parameters, 1, *time);
            set_float(&mut parameters, 0, *feedback);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Sample {
            file_name,
            samples,
            position,
            connected,
            ..
        } => {
            if let ModuleBody::Sample {
                file_name: target,
                file_missing,
                samples: target_samples,
                ..
            } = &mut module.body
            {
                *target = file_name.clone();
                *file_missing = false;
                *target_samples = Arc::new(
                    samples
                        .iter()
                        .filter_map(|sample| AudioSample::new(*sample))
                        .collect(),
                );
            }
            set_float(&mut parameters, 1, *position);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Probe { input, connected } => {
            set_float(&mut parameters, 0, *input);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::Output {
            input,
            gain,
            connected,
        } => {
            set_float(&mut parameters, 0, *input);
            set_float(&mut parameters, 1, *gain);
            apply_connected(&mut parameters, *connected);
        }
        ProjectModuleParams::DelayTap { gain } => {
            set_float(&mut parameters, 1, *gain);
        }
        ProjectModuleParams::Random { gate, connected } => {
            set_float(&mut parameters, 0, *gate);
            apply_connected(&mut parameters, *connected);
        }
    }
    for (index, parameter) in parameters.into_iter().enumerate() {
        module.body.set_parameter(index, parameter);
    }
    if let ProjectModuleParams::Delay { time, .. } = params
        && time.unit == ProjectTimeUnit::Seconds
        && let ModuleBody::Delay { time: target, .. } = &mut module.body
    {
        target.exact_seconds = Some(time.seconds.to_bits());
    }
}

fn set_float(parameters: &mut [ModuleParameter], index: usize, value: f32) {
    if let Some(parameter) = parameters.get_mut(index)
        && let ParameterValue::Float {
            value: target,
            min,
            max,
            ..
        } = &mut parameter.value
    {
        *target = (value * 100.0).round().clamp(*min as f32, *max as f32) as i32;
    }
}

fn set_time(parameters: &mut [ModuleParameter], index: usize, value: ProjectTimeValue) {
    if let Some(parameter) = parameters.get_mut(index) {
        parameter.value = time_from_project(value);
    }
}

fn apply_connected(parameters: &mut [ModuleParameter], connected: u8) {
    for (index, parameter) in parameters.iter_mut().enumerate() {
        if parameter.value.is_port() {
            parameter.connected = (connected & (1 << index)) != 0;
        }
    }
}

fn time_from_project(value: ProjectTimeValue) -> ParameterValue {
    match value.unit {
        ProjectTimeUnit::Seconds => ParameterValue::Time {
            value: (value.seconds * 100.0).round() as i32,
            unit: TimeUnit::Seconds,
        },
        ProjectTimeUnit::Samples => ParameterValue::Time {
            value: value.samples.round() as i32,
            unit: TimeUnit::Samples,
        },
        ProjectTimeUnit::Bars => ParameterValue::Bars {
            numerator: i32::from(value.bar_num.max(1)),
            denominator: i32::from(value.bar_denom.max(1)),
        },
        ProjectTimeUnit::Hz => ParameterValue::Time {
            value: value.hz.round() as i32,
            unit: TimeUnit::Hertz,
        },
    }
}

fn env_point_from_project(point: &brainwash_grid::project::EnvPoint) -> EnvPoint {
    EnvPoint {
        time: (point.time * 100.0).round().clamp(0.0, 100.0) as i32,
        value: (point.value * 100.0).round().clamp(-100.0, 100.0) as i32,
        curve: point.curve,
    }
}
