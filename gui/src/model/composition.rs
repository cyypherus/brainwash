use super::*;

pub(super) fn sync_delay_sources(surface: &mut PatchSurface) {
    let delays = surface
        .modules
        .iter()
        .filter(|module| module.kind() == ModuleKind::Delay)
        .map(|module| module.id)
        .collect::<Vec<_>>();
    for module in &mut surface.modules {
        if let ModuleBody::DelayTap { source, .. } = &mut module.body {
            source.options = delays.clone();
            if source
                .selected
                .is_none_or(|selected| !delays.contains(&selected))
            {
                source.selected = delays.first().copied();
            }
        }
        if let Some(composition) = module.composition_surface_mut() {
            sync_delay_sources(composition);
        }
    }
}

pub(super) fn composition_body(
    graph: Box<brainwash::patch::Composition>,
    next_module_id: &mut u32,
) -> Option<ModuleBody> {
    let entries = graph
        .patch()
        .module_entries()
        .map(|(id, module)| (id, module.clone()))
        .collect::<Vec<_>>();
    let connections = graph.patch().connection_entries().collect::<Vec<_>>();
    let tank = graph.name() == "FDN Tank" && entries.len() == 32;
    let mut remaining = entries.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let index = remaining.iter().position(|candidate| {
            !connections.iter().any(|(from, input)| {
                input.module() == *candidate && remaining.contains(&from.module())
            })
        })?;
        ordered.push(remaining.remove(index));
    }
    let mut modules = Vec::new();
    let ids = entries
        .iter()
        .enumerate()
        .map(|(index, (id, _))| (*id, ModuleId::new(*next_module_id + index as u32)))
        .collect::<Vec<_>>();
    *next_module_id += ids.len() as u32;
    let mut direct = graph.outputs().len() == 1
        && entries.iter().all(|(id, _)| {
            connections
                .iter()
                .filter(|(from, _)| from.module() == *id)
                .count()
                <= 1
        })
        && {
            let mut reachable = vec![graph.outputs()[0].port().module()];
            let mut index = 0;
            while index < reachable.len() {
                let target = reachable[index];
                for (from, input) in &connections {
                    if input.module() == target && !reachable.contains(&from.module()) {
                        reachable.push(from.module());
                    }
                }
                index += 1;
            }
            reachable.len() == entries.len()
        };
    let mut positions: HashMap<brainwash::patch::ModuleId, GridPos> = HashMap::new();
    let mut fallback_output_y = 0;
    if direct {
        let mut columns = HashMap::new();
        for core_id in &ordered {
            let x = connections
                .iter()
                .filter(|(_, input)| input.module() == *core_id)
                .filter_map(|(from, _)| columns.get(&from.module()))
                .map(|x| x + 1)
                .max()
                .unwrap_or(0);
            columns.insert(*core_id, x);
        }
        let mut rows = HashMap::from([(graph.outputs()[0].port().module(), 0_i32)]);
        for target_id in ordered.iter().rev() {
            let target_y = *rows
                .get(target_id)
                .expect("module reaches composition output");
            let target = entries
                .iter()
                .find_map(|(id, module)| (*id == *target_id).then_some(module))
                .expect("composition module exists");
            for (from, input) in connections
                .iter()
                .filter(|(_, input)| input.module() == *target_id)
            {
                let source = entries
                    .iter()
                    .find_map(|(id, module)| (*id == from.module()).then_some(module))
                    .expect("composition module exists");
                let source_height = source
                    .input_kinds()
                    .len()
                    .max(source.output_count() as usize)
                    .max(1) as i32;
                let source_output =
                    source_height - i32::from(source.output_count()) + i32::from(from.index());
                let target_input = target
                    .input_kinds()
                    .iter()
                    .position(|kind| *kind == input.kind())
                    .expect("composition input exists") as i32;
                rows.insert(from.module(), target_y + target_input - source_output);
            }
        }
        let shift = -rows.values().copied().min().unwrap_or(0).min(0);
        for core_id in &ordered {
            positions.insert(
                *core_id,
                GridPos::new(columns[core_id], (rows[core_id] + shift) as u16),
            );
        }
        let mut occupied = HashSet::new();
        for (core_id, node) in &entries {
            let position = positions[core_id];
            let height = node
                .input_kinds()
                .len()
                .max(node.output_count() as usize)
                .max(1) as u16;
            if (position.y..position.y + height)
                .any(|y| !occupied.insert(GridPos::new(position.x, y)))
            {
                direct = false;
                positions.clear();
                break;
            }
        }
    }
    if !direct {
        let mut y = entries
            .iter()
            .filter(|(_, node)| node.input_kinds().is_empty())
            .map(|(_, node)| node.output_count())
            .max()
            .unwrap_or(0);
        for core_id in &ordered {
            let node = entries
                .iter()
                .find_map(|(id, module)| (*id == *core_id).then_some(module))?;
            let inputs = node.input_kinds().len() as u16;
            positions.insert(*core_id, GridPos::new(0, if inputs == 0 { 0 } else { y }));
            if inputs != 0 {
                y += inputs.max(node.output_count());
            }
        }
        let mut buses = Vec::new();
        for core_id in &ordered {
            let node = entries
                .iter()
                .find_map(|(id, node)| (*id == *core_id).then_some(node))?;
            let start = positions[core_id].y;
            let mut end = connections
                .iter()
                .filter(|(from, _)| from.module() == *core_id)
                .filter_map(|(_, input)| {
                    let target = entries
                        .iter()
                        .find_map(|(id, node)| (*id == input.module()).then_some(node))?;
                    let input_index = target
                        .input_kinds()
                        .iter()
                        .position(|kind| *kind == input.kind())?;
                    Some(positions[&input.module()].y + input_index as u16)
                })
                .max()
                .unwrap_or(start)
                .max(
                    start
                        + node
                            .input_kinds()
                            .len()
                            .max(node.output_count() as usize)
                            .max(1) as u16
                        - 1,
                );
            if graph.outputs().len() == 1 && graph.outputs()[0].port().module() == *core_id {
                end = end.max(y);
            }
            let width = 1 + node.output_count().max(1);
            let mut x = connections
                .iter()
                .filter(|(_, input)| input.module() == *core_id)
                .filter_map(|(from, _)| {
                    let source = entries
                        .iter()
                        .find_map(|(id, node)| (*id == from.module()).then_some(node))?;
                    Some(positions[&from.module()].x + 1 + source.output_count().max(1))
                })
                .max()
                .unwrap_or(0);
            while buses.iter().any(|(column, columns, top, bottom)| {
                x < column + columns && x + width > *column && start <= *bottom && end >= *top
            }) {
                x += 1;
            }
            positions.get_mut(core_id)?.x = x;
            buses.push((x, width, start, end));
        }
        fallback_output_y = y;
    }
    for (core_id, node) in &entries {
        let projected = ids
            .iter()
            .find_map(|(id, projected)| (*id == *core_id).then_some(*projected))
            .expect("projected module exists");
        let exposed_input = graph
            .inputs()
            .iter()
            .find(|input| input.module() == *core_id);
        let mut body = if let Some(input) = exposed_input {
            let default = match node {
                AudioModule::Input { default, .. } => default.value(),
                _ => 0.0,
            };
            let mut value = float_param(-100_000, 100_000, 1, (default * 100.0).round() as i32);
            value.value = AudioSample::new(default)?;
            ModuleBody::CompositionInput {
                label: input.label().to_string(),
                kind: input.kind(),
                value,
            }
        } else {
            match node {
                AudioModule::Composition(composition) => {
                    composition_body(composition.clone(), next_module_id)?
                }
                AudioModule::DelayTap(tap) => {
                    let source = ids
                        .iter()
                        .find_map(|(id, projected)| (*id == tap.delay()).then_some(*projected))
                        .expect("delay tap source exists");
                    let options = ids
                        .iter()
                        .filter_map(|(id, projected)| {
                            entries
                                .iter()
                                .find_map(|(candidate, module)| {
                                    (*candidate == *id).then_some(module)
                                })
                                .is_some_and(|module| matches!(module, AudioModule::Delay { .. }))
                                .then_some(*projected)
                        })
                        .collect();
                    let mut body = ModuleKind::DelayTap.default_body();
                    let ModuleBody::DelayTap {
                        source: selected,
                        gain,
                    } = &mut body
                    else {
                        unreachable!()
                    };
                    selected.selected = Some(source);
                    selected.options = options;
                    gain.value = AudioSample::new(tap.gain().value())?;
                    body
                }
                _ => graph_node_body(node),
            }
        };
        if let ModuleBody::Delay { feedback, .. } = &mut body
            && !connections.iter().any(|(_, input)| {
                input.module() == *core_id && input.kind() == AudioInputKind::Feedback
            })
        {
            feedback.connected = false;
        }
        let index = entries.iter().position(|(id, _)| id == core_id)?;
        let position = if tank {
            match index {
                0 => GridPos::new(8, 0),
                1 => GridPos::new(8, 1),
                2 => GridPos::new(9, 5),
                3 => GridPos::new(9, 4),
                4 => GridPos::new(9, 1),
                5..=20 if index % 2 == 1 => {
                    let channel = (index - 5) / 2;
                    GridPos::new(
                        21 + 2 * (channel % 4) as u16,
                        1 + 10 * (channel / 4) as u16 + 2 * (channel % 4) as u16,
                    )
                }
                5..=20 => GridPos::new(((index - 6) / 2) as u16, 0),
                21..=28 => {
                    let channel = index - 21;
                    GridPos::new(18, 1 + 10 * (channel / 4) as u16 + (channel % 4) as u16)
                }
                29 => GridPos::new(20, 1),
                30 => GridPos::new(20, 11),
                31 => GridPos::new(0, 1),
                _ => unreachable!(),
            }
        } else {
            *positions.get(core_id)?
        };
        modules.push(Module {
            id: projected,
            position,
            orientation: if tank && (index == 31 || (6..=20).contains(&index) && index % 2 == 0) {
                Orientation::Down
            } else {
                Orientation::Right
            },
            body,
            disabled: false,
        });
    }
    let mut exposed_outputs: Vec<(brainwash::patch::OutputPort, GridPos)> = Vec::new();
    let output_x = modules
        .iter()
        .map(|module| {
            module.position.x
                + module_footprint(module, module.composition_surface().map(composition_ports)).0
        })
        .max()?;
    for declared in graph.outputs() {
        let source = ids
            .iter()
            .find_map(|(id, projected)| (declared.port().module() == *id).then_some(*projected))?;
        let source_module = modules.iter().find(|module| module.id == source)?;
        let adjacent_output = direct
            || source_module.output_count() == 1
                && !connections.iter().any(|(from, _)| *from == declared.port());
        let position = if tank {
            GridPos::new(7, 3)
        } else if graph.outputs().len() == 1 && adjacent_output {
            let source_ports = source_module.composition_surface().map(composition_ports);
            let (_, source_height) = module_footprint(source_module, source_ports);
            GridPos::new(
                source_module.position.x + module_footprint(source_module, source_ports).0,
                source_module.position.y + source_height
                    - source_ports
                        .map(|(_, outputs)| outputs)
                        .unwrap_or_else(|| source_module.output_count())
                    + declared.port().index(),
            )
        } else if graph.outputs().len() == 1 {
            let source_ports = source_module.composition_surface().map(composition_ports);
            let (_, source_height) = module_footprint(source_module, source_ports);
            GridPos::new(
                source_module.position.x
                    + module_footprint(source_module, source_ports).0
                    + declared.port().index(),
                fallback_output_y.max(
                    source_module.position.y + source_height
                        - source_ports
                            .map(|(_, outputs)| outputs)
                            .unwrap_or_else(|| source_module.output_count())
                        + 1,
                ),
            )
        } else {
            let source_ports = source_module.composition_surface().map(composition_ports);
            let (_, source_height) = module_footprint(source_module, source_ports);
            GridPos::new(
                output_x + 1,
                source_module.position.y + source_height
                    - source_ports
                        .map(|(_, outputs)| outputs)
                        .unwrap_or_else(|| source_module.output_count())
                    + declared.port().index(),
            )
        };
        modules.push(Module {
            id: ModuleId::new(*next_module_id),
            position,
            orientation: if tank || graph.outputs().len() == 1 && !adjacent_output {
                Orientation::Down
            } else {
                Orientation::Right
            },
            body: ModuleBody::CompositionOutput {
                label: declared.label().to_string(),
                input: signal_param(),
            },
            disabled: false,
        });
        exposed_outputs.push((declared.port(), position));
        *next_module_id += 1;
    }
    if tank {
        for (x, y, body) in [
            (6, 6, ModuleBody::TurnDownRight),
            (13, 6, ModuleBody::TurnRightDown),
            (13, 9, ModuleBody::TopSplit),
            (13, 19, ModuleBody::TurnDownRight),
            (17, 4, ModuleBody::TurnRightDown),
            (17, 5, ModuleBody::TopSplit),
            (17, 15, ModuleBody::TurnDownRight),
            (16, 5, ModuleBody::TurnRightDown),
            (16, 6, ModuleBody::TopSplit),
            (16, 16, ModuleBody::TurnDownRight),
            (15, 3, ModuleBody::TurnRightDown),
            (15, 7, ModuleBody::TopSplit),
            (15, 17, ModuleBody::TurnDownRight),
            (14, 0, ModuleBody::TurnRightDown),
            (14, 8, ModuleBody::TopSplit),
            (14, 18, ModuleBody::TurnDownRight),
        ] {
            modules.push(Module {
                id: ModuleId::new(*next_module_id),
                position: GridPos::new(x, y),
                orientation: Orientation::Right,
                body,
                disabled: false,
            });
            *next_module_id += 1;
        }
    } else {
        for (core_source, projected_source) in &ids {
            let source = modules
                .iter()
                .find(|module| module.id == *projected_source)
                .expect("projected source exists");
            let source_position = source.position;
            let (source_width, source_height) =
                module_footprint(source, source.composition_surface().map(composition_ports));
            let core_module = entries
                .iter()
                .find_map(|(id, module)| (*id == *core_source).then_some(module))?;
            for source_output in 0..core_module.output_count() {
                let source_y =
                    source_position.y + source_height - core_module.output_count() + source_output;
                let source_x = source_position.x;
                let mut rows = connections
                    .iter()
                    .filter(|(from, _)| {
                        from.module() == *core_source && from.index() == source_output
                    })
                    .filter_map(|(_, input)| {
                        let target = ids.iter().find_map(|(id, projected)| {
                            (*id == input.module()).then_some(*projected)
                        })?;
                        let target_module = modules.iter().find(|module| module.id == target)?;
                        let core_target = entries
                            .iter()
                            .find_map(|(id, module)| (*id == input.module()).then_some(module))?;
                        let input_index = core_target
                            .input_kinds()
                            .iter()
                            .position(|kind| *kind == input.kind())?;
                        Some(target_module.position.y + input_index as u16)
                    })
                    .collect::<Vec<_>>();
                let output_positions = exposed_outputs
                    .iter()
                    .filter_map(|(port, position)| {
                        (port.module() == *core_source && port.index() == source_output)
                            .then_some(*position)
                    })
                    .collect::<Vec<_>>();
                let bus_x = source_x + source_width + source_output;
                let feeds_output_below = output_positions
                    .iter()
                    .any(|position| position.x == bus_x && position.y > source_y);
                rows.extend(
                    output_positions
                        .iter()
                        .filter(|position| position.x != bus_x)
                        .map(|position| position.y),
                );
                rows.sort_unstable();
                rows.dedup();
                if rows.len() == 1 && rows[0] == source_y && !feeds_output_below {
                    continue;
                }
                let Some(last) = rows
                    .last()
                    .copied()
                    .or(feeds_output_below.then_some(source_y))
                else {
                    continue;
                };
                let same_row = rows.first().is_some_and(|row| *row == source_y);
                modules.push(Module {
                    id: ModuleId::new(*next_module_id),
                    position: GridPos::new(bus_x, source_y),
                    orientation: Orientation::Right,
                    body: if same_row {
                        ModuleBody::LeftSplit
                    } else {
                        ModuleBody::TurnRightDown
                    },
                    disabled: false,
                });
                *next_module_id += 1;
                for row in rows.into_iter().filter(|row| *row > source_y) {
                    modules.push(Module {
                        id: ModuleId::new(*next_module_id),
                        position: GridPos::new(bus_x, row),
                        orientation: Orientation::Right,
                        body: if row == last {
                            ModuleBody::TurnDownRight
                        } else {
                            ModuleBody::TopSplit
                        },
                        disabled: false,
                    });
                    *next_module_id += 1;
                }
            }
        }
    }
    Some(ModuleBody::Composition {
        name: graph.name().to_string(),
        surface: PatchSurface {
            cursor: GridPos::new(0, 0),
            modules,
        },
        category: ModuleCategory::Composition,
    })
}

fn graph_node_body(module: &AudioModule) -> ModuleBody {
    match module {
        AudioModule::Delay {
            input,
            time,
            feedback,
        } => ModuleBody::Delay {
            input: float_param(-100_000, 100_000, 1, (input.value() * 100.0).round() as i32),
            time: match time {
                Duration::Samples(samples) => time_param(samples.value() as i32, TimeUnit::Samples),
                Duration::Seconds(seconds) => {
                    let mut time =
                        time_param((seconds.value() * 100.0).round() as i32, TimeUnit::Seconds);
                    time.exact_seconds = Some(seconds.value().to_bits());
                    time
                }
            },
            feedback: float_param(0, 99, 1, (feedback.value() * 100.0).round() as i32),
        },
        _ => ModuleBody::Primitive(module.clone()),
    }
}

pub(super) fn graph_node_label(module: &AudioModule) -> &'static str {
    match module {
        AudioModule::Input { .. } => "Input",
        AudioModule::Freq => "Frequency",
        AudioModule::Gate => "Gate",
        AudioModule::Degree => "Degree",
        AudioModule::Expression => "Expression",
        AudioModule::Constant(_) => "Constant",
        AudioModule::Unary { op, .. } => match op {
            brainwash::patch::UnaryOp::Absolute => "Absolute",
            brainwash::patch::UnaryOp::Sine => "Sine",
            brainwash::patch::UnaryOp::HyperbolicTangent => "Tanh",
            brainwash::patch::UnaryOp::Arctangent => "Atan",
            brainwash::patch::UnaryOp::Exponential => "Exp",
            brainwash::patch::UnaryOp::Sign => "Sign",
        },
        AudioModule::Damp { .. } => "Damping",
        AudioModule::Phase { .. } => "Phase",
        AudioModule::Oscillator { .. } => "Oscillator",
        AudioModule::Saturation { .. } => "Saturation",
        AudioModule::Noise => "Noise",
        AudioModule::Rise { .. } => "Rise",
        AudioModule::Fall { .. } => "Fall",
        AudioModule::Ramp { .. } => "Ramp",
        AudioModule::Envelope { .. } => "Envelope",
        AudioModule::Filter { .. } => "Filter",
        AudioModule::Comb { .. } => "Comb",
        AudioModule::Allpass { .. } => "Allpass",
        AudioModule::Delay { .. } => "Delay",
        AudioModule::DelayTap { .. } => "Delay Tap",
        AudioModule::Slew { .. } => "Slew",
        AudioModule::Binary { op, .. } => match op {
            BinaryOp::Multiply => "Multiply",
            BinaryOp::Add => "Add",
            BinaryOp::Subtract => "Subtract",
            BinaryOp::Divide => "Divide",
            BinaryOp::Power => "Power",
            BinaryOp::Remainder => "Remainder",
            BinaryOp::Minimum => "Minimum",
            BinaryOp::Maximum => "Maximum",
            BinaryOp::GreaterThan => "Greater Than",
            BinaryOp::LessThan => "Less Than",
            BinaryOp::Equal => "Equal",
        },
        AudioModule::Switch { .. } => "Switch",
        AudioModule::Random { .. } => "Random",
        AudioModule::Sample { .. } => "Sample",
        AudioModule::Probe { .. } => "Probe",
        AudioModule::Composition(_) => "Composition",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_connections(
        state: &GuiState,
        graph: &brainwash::patch::Composition,
        surface: &PatchSurface,
    ) -> (usize, usize) {
        let core = graph.patch().module_entries().collect::<Vec<_>>();
        let gui = surface
            .modules
            .iter()
            .filter(|module| !module.is_wire() && module.kind() != ModuleKind::CompositionOutput)
            .collect::<Vec<_>>();
        let expected = graph
            .patch()
            .connection_entries()
            .map(|(from, to)| {
                (
                    core.iter()
                        .position(|(id, _)| *id == from.module())
                        .unwrap(),
                    from.index(),
                    core.iter().position(|(id, _)| *id == to.module()).unwrap(),
                    to.kind(),
                )
            })
            .collect::<HashSet<_>>();
        let actual = state
            .semantic_surface_connections(&surface.modules)
            .unwrap()
            .into_iter()
            .filter_map(|connection| {
                Some((
                    gui.iter().position(|module| module.id == connection.from)?,
                    connection.output as u16,
                    gui.iter().position(|module| module.id == connection.to)?,
                    connection.audio_input(),
                ))
            })
            .collect::<HashSet<_>>();
        assert_eq!(actual, expected, "{} connections", graph.name());
        let mut counts = (
            surface.modules.len(),
            surface
                .modules
                .iter()
                .filter(|module| module.is_wire())
                .count(),
        );
        for ((_, core), gui) in core.into_iter().zip(gui) {
            if let (AudioModule::Composition(nested), Some(surface)) =
                (core, gui.composition_surface())
            {
                let nested = check_connections(state, nested, surface);
                counts.0 += nested.0;
                counts.1 += nested.1;
            }
        }
        counts
    }

    #[test]
    fn projected_compositions_preserve_their_signal_paths() {
        for graph in [
            brainwash::preset::reverb(
                Unit::new(0.7).unwrap(),
                Unit::new(0.2).unwrap(),
                Unit::ZERO,
                Unit::new(0.8).unwrap(),
            ),
            brainwash::preset::adsr(Unit::new(0.25).unwrap(), Unit::new(0.7).unwrap()),
            brainwash::preset::flanger(
                Hertz::new(0.2).unwrap(),
                Unit::new(0.3).unwrap(),
                Unit::new(0.5).unwrap(),
            ),
            brainwash::preset::compressor(
                Unit::new(0.5).unwrap(),
                brainwash::patch::CompressorRatio::new(4.0).unwrap(),
                Seconds::new(0.01).unwrap(),
                Seconds::new(0.1).unwrap(),
                brainwash::patch::Gain::new(1.0).unwrap(),
            ),
        ] {
            let AudioModule::Composition(graph) = graph else {
                unreachable!()
            };
            let mut state = GuiState::default();
            let owner = Module {
                id: ModuleId::new(0),
                position: GridPos::new(0, 0),
                orientation: Orientation::Right,
                body: composition_body(graph.clone(), &mut state.next_module_id).unwrap(),
                disabled: false,
            };
            let rate = SampleRate::new(44_100).unwrap();
            let projected = audio_patch::gui_composition(&state, &owner, rate, 120).unwrap();
            let surface = owner.composition_surface().unwrap();
            let counts = check_connections(&state, &graph, surface);
            eprintln!(
                "{}: {} modules, {} routing pieces, {:?}; recursively {} modules, {} routing pieces",
                graph.name(),
                surface.modules.len(),
                surface
                    .modules
                    .iter()
                    .filter(|module| module.is_wire())
                    .count(),
                surface_extent(&surface.modules),
                counts.0,
                counts.1
            );
            let input_kind = graph.inputs()[0].kind();
            let mut patches = [AudioModule::Composition(graph), projected].map(|module| {
                let mut patch = Patch::new();
                let input = patch.insert(AudioModule::Gate);
                let effect = patch.insert(module);
                patch
                    .connect_input(
                        patch.output_port(input, 0).unwrap(),
                        patch.input_port(effect, input_kind).unwrap(),
                    )
                    .unwrap();
                patch.output(patch.output_port(effect, 0).unwrap()).unwrap();
                CompiledPatch::new(&patch, rate).unwrap()
            });
            for frame in 0..12_000 {
                let controls = brainwash::compile::PatchControls {
                    gate: if frame % 100 < 50 { 1.0 } else { 0.0 },
                    ..Default::default()
                };
                let expected = patches[0].next_with_controls(controls);
                let actual = patches[1].next_with_controls(controls);
                assert_eq!(actual, expected, "frame {frame}");
            }
        }
    }
}
