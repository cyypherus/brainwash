use super::*;

impl GuiState {
    pub fn compile_audio_patch(&self, rate: SampleRate) -> Result<CompiledPatch, AudioPatchError> {
        self.compile_gui_audio_patch(rate).map(|audio| audio.patch)
    }

    pub(super) fn compile_gui_audio_patch(
        &self,
        rate: SampleRate,
    ) -> Result<GuiAudioPatch, AudioPatchError> {
        let instrument = self.instrument();
        let root_modules = &instrument.root.modules;
        let output = self
            .instrument()
            .root
            .modules
            .iter()
            .find(|module| !module.disabled && module.kind() == ModuleKind::Output)
            .ok_or(AudioPatchError::MissingOutput)?;
        let connections = self.semantic_surface_connections(root_modules)?;
        let mut needed = Vec::new();
        let mut has_output_signal = false;
        for connection in connections
            .iter()
            .filter(|connection| connection.to == output.id)
        {
            let input = connection.input();
            if input >= self.module_input_count(output) as usize {
                continue;
            }
            if !self.module_input_connected(output, input as u16) {
                continue;
            }
            if input == 0 {
                has_output_signal = true;
            }
            collect_audio_inputs(connection.from, &connections, &mut needed);
        }
        if !has_output_signal {
            return Err(AudioPatchError::MissingOutput);
        }
        let output_dependencies = OutputDependencies(needed.clone());
        for module in root_modules
            .iter()
            .filter(|module| !module.disabled && module.kind() == ModuleKind::Probe)
        {
            collect_audio_inputs(module.id, &connections, &mut needed);
        }
        for owner in root_modules
            .iter()
            .filter(|module| !module.disabled && module.is_composition())
        {
            if owner.composition_surface().is_some_and(|surface| {
                surface
                    .modules
                    .iter()
                    .any(|module| !module.disabled && module.kind() == ModuleKind::Probe)
            }) {
                collect_audio_inputs(owner.id, &connections, &mut needed);
            }
        }
        let voice_mode = output_voice_mode(root_modules, &output_dependencies);

        let mut patch = Patch::new();
        let mut ids = Vec::new();
        let mut probes = Vec::new();
        let mut meters = Vec::new();
        for id in needed.iter().rev().copied() {
            let module = root_modules
                .iter()
                .find(|module| !module.disabled && module.id == id)
                .ok_or(AudioPatchError::Compile(CompileError::MissingModule))?;
            if module.is_composition() || module.kind() == ModuleKind::DelayTap {
                continue;
            }
            let audio = audio_module(module, rate, self.bpm)?;
            let audio_id = patch.insert(audio);
            if module.kind() == ModuleKind::Probe {
                probes.push(ProbeRoute {
                    source: audio_id,
                    target: module.id,
                });
            }
            let input_count = self.module_input_count(module);
            if self.show_meters && input_count > 0 {
                if let Some(route) = MeterRoute::new(audio_id, module.id, input_count as usize) {
                    meters.push(route);
                }
            }
            ids.push(AudioNode {
                key: AudioKey::Root(id),
                id: audio_id,
            });
        }
        for id in needed.iter().rev().copied() {
            let Some(module) = root_modules.iter().find(|module| {
                !module.disabled && module.id == id && module.kind() == ModuleKind::DelayTap
            }) else {
                continue;
            };
            let audio_id = insert_audio_delay_tap(&mut patch, module, root_modules, &ids, |id| {
                AudioKey::Root(id)
            })?;
            ids.push(AudioNode {
                key: AudioKey::Root(id),
                id: audio_id,
            });
        }

        for (owner_id, composition) in needed
            .iter()
            .filter_map(|id| {
                root_modules
                    .iter()
                    .find(|module| module.id == *id && module.is_composition())
            })
            .filter_map(|module| {
                module
                    .composition_surface()
                    .map(|surface| (module.id, surface))
            })
        {
            for module in composition.modules.iter().filter(|module| {
                !module.disabled
                    && !module.is_wire()
                    && module.kind() != ModuleKind::CompositionOutput
            }) {
                if module.kind() == ModuleKind::DelayTap {
                    continue;
                }
                let audio = if module.is_composition() {
                    gui_composition(self, module, rate, self.bpm)?
                } else {
                    audio_module(module, rate, self.bpm)?
                };
                let audio_id = patch.insert(audio);
                if module.kind() == ModuleKind::Probe {
                    probes.push(ProbeRoute {
                        source: audio_id,
                        target: module.id,
                    });
                }
                let input_count = self.module_input_count(module);
                if self.show_meters && input_count > 0 {
                    if let Some(route) = MeterRoute::new(audio_id, module.id, input_count as usize)
                    {
                        meters.push(route);
                    }
                }
                ids.push(AudioNode {
                    key: AudioKey::Composition {
                        owner: owner_id,
                        module: module.id,
                    },
                    id: audio_id,
                });
            }
            for module in composition
                .modules
                .iter()
                .filter(|module| !module.disabled && module.kind() == ModuleKind::DelayTap)
            {
                let audio_id =
                    insert_audio_delay_tap(&mut patch, module, &composition.modules, &ids, |id| {
                        AudioKey::Composition {
                            owner: owner_id,
                            module: id,
                        }
                    })?;
                ids.push(AudioNode {
                    key: AudioKey::Composition {
                        owner: owner_id,
                        module: module.id,
                    },
                    id: audio_id,
                });
            }
        }

        let output_gain = audio_float(output, 1)?;
        let output_id = patch.insert(AudioModule::Binary {
            op: BinaryOp::Multiply,
            a: AudioSample::ZERO,
            b: AudioSample::new(output_gain).ok_or(AudioPatchError::InvalidParameter)?,
        });

        for connection in &connections {
            let from = root_connection_source(self, root_modules, &ids, connection);
            if connection.to == output.id {
                let input = connection.input();
                if input >= self.module_input_count(output) as usize {
                    continue;
                }
                if !self.module_input_connected(output, input as u16) {
                    continue;
                }
                let Some(from) = from else {
                    if input == 0 {
                        return Err(AudioPatchError::MissingOutput);
                    }
                    continue;
                };
                let port = patch
                    .input_port(output_id, connection.audio_input())
                    .map_err(AudioPatchError::Connect)?;
                let from = patch
                    .output_port(from.0, from.1)
                    .map_err(AudioPatchError::Connect)?;
                patch
                    .connect_input(from, port)
                    .map_err(AudioPatchError::Connect)?;
                continue;
            }
            let Some(target) = root_modules
                .iter()
                .find(|module| module.id == connection.to)
            else {
                continue;
            };
            let input = connection.input();
            if input >= self.module_input_count(target) as usize {
                continue;
            }
            if !self.module_input_connected(target, input as u16) {
                continue;
            }
            if target.is_composition() {
                let Some(from) = from else {
                    continue;
                };
                let Some(composition) = target.composition_surface() else {
                    continue;
                };
                let inputs = composition_inputs(composition);
                let Some(input_id) = inputs.get(input).copied() else {
                    continue;
                };
                let Some(to) = audio_id(
                    &ids,
                    AudioKey::Composition {
                        owner: target.id,
                        module: input_id,
                    },
                ) else {
                    continue;
                };
                let port = patch
                    .input_port(to, AudioInputKind::A)
                    .map_err(AudioPatchError::Connect)?;
                let from = patch
                    .output_port(from.0, from.1)
                    .map_err(AudioPatchError::Connect)?;
                patch
                    .connect_input(from, port)
                    .map_err(AudioPatchError::Connect)?;
            } else {
                let Some(from) = from else {
                    continue;
                };
                let Some(to) = audio_id(&ids, AudioKey::Root(connection.to)) else {
                    continue;
                };
                let port = patch
                    .input_port(to, connection.audio_input())
                    .map_err(AudioPatchError::Connect)?;
                let from = patch
                    .output_port(from.0, from.1)
                    .map_err(AudioPatchError::Connect)?;
                patch
                    .connect_input(from, port)
                    .map_err(AudioPatchError::Connect)?;
            }
        }

        for (owner, composition) in needed.iter().filter_map(|id| {
            root_modules
                .iter()
                .find(|module| module.id == *id)
                .and_then(|module| module.composition_surface().map(|surface| (*id, surface)))
        }) {
            for connection in self.semantic_surface_connections(&composition.modules)? {
                let Some(from) = audio_id(
                    &ids,
                    AudioKey::Composition {
                        owner,
                        module: connection.from,
                    },
                ) else {
                    continue;
                };
                let Some(target) = composition
                    .modules
                    .iter()
                    .find(|module| module.id == connection.to)
                else {
                    continue;
                };
                let Some(to) = audio_id(
                    &ids,
                    AudioKey::Composition {
                        owner,
                        module: connection.to,
                    },
                ) else {
                    continue;
                };
                let input = connection.input();
                if input >= self.module_input_count(target) as usize {
                    continue;
                }
                if !self.module_input_connected(target, input as u16) {
                    continue;
                }
                let port = patch
                    .input_port(to, connection.audio_input())
                    .map_err(AudioPatchError::Connect)?;
                let from = patch
                    .output_port(from, 0)
                    .map_err(AudioPatchError::Connect)?;
                patch
                    .connect_input(from, port)
                    .map_err(AudioPatchError::Connect)?;
            }
        }

        let output = patch
            .output_port(output_id, 0)
            .map_err(AudioPatchError::Connect)?;
        patch.output(output).map_err(AudioPatchError::Connect)?;
        let patch = CompiledPatch::new(&patch, rate).map_err(AudioPatchError::Compile)?;
        Ok(GuiAudioPatch {
            patch,
            probes,
            meters,
            voice_mode,
        })
    }
}

pub(super) fn collect_audio_inputs(
    module: ModuleId,
    connections: &[Connection],
    needed: &mut Vec<ModuleId>,
) {
    if needed.contains(&module) {
        return;
    }
    needed.push(module);
    for connection in connections {
        if connection.to == module {
            collect_audio_inputs(connection.from, connections, needed);
        }
    }
}

pub(super) fn output_voice_mode(
    modules: &[Module],
    dependencies: &OutputDependencies,
) -> VoiceMode {
    if modules.iter().any(|module| {
        dependencies.0.contains(&module.id) && module_uses_voice_controls(module.kind())
    }) || dependencies
        .0
        .iter()
        .filter_map(|id| {
            modules
                .iter()
                .find(|module| module.id == *id)
                .and_then(Module::composition_surface)
        })
        .any(|surface| {
            surface
                .modules
                .iter()
                .any(|module| !module.disabled && module_uses_voice_controls(module.kind()))
        })
    {
        VoiceMode::Polyphonic
    } else {
        VoiceMode::Single
    }
}

fn module_uses_voice_controls(kind: ModuleKind) -> bool {
    matches!(
        kind,
        ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree | ModuleKind::DegreeGate
    )
}

pub(super) fn audio_patch_error_message(error: &AudioPatchError) -> &'static str {
    match error {
        AudioPatchError::MissingOutput => "connect signal to Output",
        AudioPatchError::InvalidParameter => "invalid parameter",
        AudioPatchError::Connect(_) => "invalid connection",
        AudioPatchError::Compile(CompileError::Cycle) => "cycle",
        AudioPatchError::Compile(CompileError::InvalidDelay) => "invalid delay",
        AudioPatchError::Compile(CompileError::InvalidInput) => "invalid input",
        AudioPatchError::Compile(CompileError::MissingModule) => "missing module",
        AudioPatchError::Compile(CompileError::MissingOutput) => "missing output",
    }
}

pub(super) fn gui_composition(
    state: &GuiState,
    owner: &Module,
    rate: SampleRate,
    bpm: u16,
) -> Result<AudioModule, AudioPatchError> {
    let ModuleBody::Composition { name, surface } = &owner.body else {
        return Err(AudioPatchError::InvalidParameter);
    };
    let mut patch = Patch::new();
    let mut ids = Vec::new();
    let connections = state.semantic_surface_connections(&surface.modules)?;
    for module in surface.modules.iter().filter(|module| {
        !module.disabled && !module.is_wire() && module.kind() != ModuleKind::CompositionOutput
    }) {
        let audio_id = match &module.body {
            ModuleBody::CompositionInput { kind, value, .. } => patch.insert(AudioModule::Input {
                kind: *kind,
                default: AudioSample::new(value.value as f32 / 100.0)
                    .ok_or(AudioPatchError::InvalidParameter)?,
            }),
            ModuleBody::Composition { .. } => {
                patch.insert(gui_composition(state, module, rate, bpm)?)
            }
            ModuleBody::DelayTap { .. } => {
                insert_audio_delay_tap(&mut patch, module, &surface.modules, &ids, AudioKey::Root)?
            }
            _ => patch.insert(audio_module(module, rate, bpm)?),
        };
        ids.push(AudioNode {
            key: AudioKey::Root(module.id),
            id: audio_id,
        });
    }
    for connection in &connections {
        let Some(from) = audio_id(&ids, AudioKey::Root(connection.from)) else {
            continue;
        };
        let Some(to) = audio_id(&ids, AudioKey::Root(connection.to)) else {
            continue;
        };
        let input = connection.audio_input();
        let port = patch
            .input_port(to, input)
            .map_err(AudioPatchError::Connect)?;
        let output = surface
            .modules
            .iter()
            .find(|module| module.id == connection.from)
            .is_some_and(Module::is_composition)
            .then_some(connection.output as u16)
            .unwrap_or(0);
        let from = patch
            .output_port(from, output)
            .map_err(AudioPatchError::Connect)?;
        patch
            .connect_input(from, port)
            .map_err(AudioPatchError::Connect)?;
    }
    let outputs = composition_outputs(surface)
        .into_iter()
        .map(|id| {
            let module = surface
                .modules
                .iter()
                .find(|module| module.id == id)
                .ok_or(AudioPatchError::MissingOutput)?;
            let ModuleBody::CompositionOutput { label, .. } = &module.body else {
                return Err(AudioPatchError::MissingOutput);
            };
            let connection = connections
                .iter()
                .find(|connection| connection.to == id)
                .ok_or(AudioPatchError::MissingOutput)?;
            let output = audio_id(&ids, AudioKey::Root(connection.from))
                .ok_or(AudioPatchError::MissingOutput)?;
            let output_index = surface
                .modules
                .iter()
                .find(|module| module.id == connection.from)
                .is_some_and(Module::is_composition)
                .then_some(connection.output as u16)
                .unwrap_or(0);
            patch
                .output_port(output, output_index)
                .map(|output| (label.clone(), output))
                .map_err(AudioPatchError::Connect)
        })
        .collect::<Result<Vec<_>, _>>()?;
    patch
        .output(outputs.first().ok_or(AudioPatchError::MissingOutput)?.1)
        .map_err(AudioPatchError::Connect)?;
    let inputs = composition_inputs(surface)
        .into_iter()
        .filter_map(|id| {
            let module = surface.modules.iter().find(|module| module.id == id)?;
            let ModuleBody::CompositionInput { label, kind, .. } = &module.body else {
                return None;
            };
            Some((label.clone(), *kind, audio_id(&ids, AudioKey::Root(id))?))
        })
        .collect::<Vec<_>>();
    brainwash::patch::Composition::new(name.clone(), patch, inputs, outputs)
        .map(|composition| AudioModule::Composition(Box::new(composition)))
        .map_err(|_| AudioPatchError::InvalidParameter)
}

pub(super) fn insert_audio_delay_tap(
    patch: &mut Patch,
    module: &Module,
    modules: &[Module],
    ids: &[AudioNode],
    key: impl Fn(ModuleId) -> AudioKey,
) -> Result<AudioModuleId, AudioPatchError> {
    let ModuleBody::DelayTap { source, .. } = &module.body else {
        return Err(AudioPatchError::InvalidParameter);
    };
    let delay = source
        .selected
        .and_then(|selected| {
            modules
                .iter()
                .find(|candidate| candidate.id == selected && candidate.kind() == ModuleKind::Delay)
        })
        .and_then(|delay| audio_id(ids, key(delay.id)))
        .ok_or(AudioPatchError::InvalidParameter)?;
    patch
        .insert_delay_tap(delay, audio_unit(module, 1)?)
        .map_err(AudioPatchError::Connect)
}

pub(super) fn audio_module(
    module: &Module,
    rate: SampleRate,
    bpm: u16,
) -> Result<AudioModule, AudioPatchError> {
    if let ModuleBody::Primitive(module) = &module.body {
        return Ok(module.clone());
    }
    match module.kind() {
        ModuleKind::Primitive => unreachable!(),
        ModuleKind::Freq => Ok(AudioModule::Freq),
        ModuleKind::Gate => Ok(AudioModule::Gate),
        ModuleKind::Degree => Ok(AudioModule::Degree),
        ModuleKind::DegreeGate => Ok(AudioModule::DegreeGate {
            target: audio_int(module, 0)?,
        }),
        ModuleKind::Rate => Ok(AudioModule::Constant(
            AudioSample::new(audio_rate(module, 0, rate, bpm)?)
                .ok_or(AudioPatchError::InvalidParameter)?,
        )),
        ModuleKind::Transpose => Ok(brainwash::preset::transpose(audio_sample(module, 1)?)),
        ModuleKind::Osc => Ok(AudioModule::Osc {
            wave: audio_wave(module)?,
            frequency: Hertz::new(audio_float(module, 1)?)
                .ok_or(AudioPatchError::InvalidParameter)?,
        }),
        ModuleKind::Rise => Ok(AudioModule::Rise {
            time: audio_duration(module, 1, rate, bpm)?,
        }),
        ModuleKind::Fall => Ok(AudioModule::Fall {
            time: audio_duration(module, 1, rate, bpm)?,
        }),
        ModuleKind::Ramp => Ok(AudioModule::Ramp {
            value: audio_sample(module, 0)?,
            time: audio_duration(module, 1, rate, bpm)?,
        }),
        ModuleKind::Adsr => Ok(brainwash::preset::adsr(
            audio_unit(module, 2)?,
            audio_unit(module, 3)?,
        )),
        ModuleKind::Envelope => Ok(AudioModule::Envelope {
            points: Arc::new(
                module
                    .env_points()
                    .iter()
                    .map(|point| {
                        Ok(AudioEnvPoint {
                            time: Unit::new(point.time as f32 / 100.0)
                                .ok_or(AudioPatchError::InvalidParameter)?,
                            value: AudioSample::new(point.value as f32 / 100.0)
                                .ok_or(AudioPatchError::InvalidParameter)?,
                            curve: point.curve,
                        })
                    })
                    .collect::<Result<Vec<_>, AudioPatchError>>()?,
            ),
        }),
        ModuleKind::Lowpass => Ok(AudioModule::Lowpass {
            cutoff: filter_cutoff(audio_float(module, 1)?)?,
            resonance: Resonance::new(audio_float(module, 2)?)
                .ok_or(AudioPatchError::InvalidParameter)?,
        }),
        ModuleKind::Highpass => Ok(AudioModule::Highpass {
            cutoff: filter_cutoff(audio_float(module, 1)?)?,
            resonance: Resonance::new(audio_float(module, 2)?)
                .ok_or(AudioPatchError::InvalidParameter)?,
        }),
        ModuleKind::Comb => Ok(AudioModule::Comb {
            time: audio_duration(module, 1, rate, bpm)?,
            feedback: audio_unit(module, 2)?,
            damp: audio_unit(module, 3)?,
        }),
        ModuleKind::Allpass => Ok(AudioModule::Allpass {
            time: audio_duration(module, 1, rate, bpm)?,
            feedback: audio_unit(module, 2)?,
        }),
        ModuleKind::Delay => {
            let ModuleBody::Delay { time, .. } = &module.body else {
                unreachable!()
            };
            Ok(AudioModule::Delay {
                time: if let Some(seconds) = time.exact_seconds {
                    Duration::Seconds(
                        Seconds::new(f32::from_bits(seconds))
                            .ok_or(AudioPatchError::InvalidParameter)?,
                    )
                } else {
                    audio_duration(module, 1, rate, bpm)?
                },
                feedback: Unit::ZERO,
            })
        }
        ModuleKind::DelayTap => Err(AudioPatchError::InvalidParameter),
        ModuleKind::Reverb => Ok(brainwash::preset::reverb(
            audio_unit(module, 1)?,
            audio_unit(module, 2)?,
            audio_unit(module, 3)?,
            audio_unit(module, 4)?,
        )),
        ModuleKind::Distortion => Ok(brainwash::preset::distortion(
            audio_distortion(module)?,
            Drive::new(audio_float(module, 2)?).ok_or(AudioPatchError::InvalidParameter)?,
            Sample::new(audio_float(module, 3)?).ok_or(AudioPatchError::InvalidParameter)?,
        )),
        ModuleKind::Compressor => Ok(brainwash::preset::compressor(
            audio_unit(module, 1)?,
            CompressorRatio::new(audio_float(module, 2)?)
                .ok_or(AudioPatchError::InvalidParameter)?,
            Seconds::new(audio_float(module, 3)?).ok_or(AudioPatchError::InvalidParameter)?,
            Seconds::new(audio_float(module, 4)?).ok_or(AudioPatchError::InvalidParameter)?,
            Gain::new(audio_float(module, 5)?).ok_or(AudioPatchError::InvalidParameter)?,
        )),
        ModuleKind::Flanger => Ok(brainwash::preset::flanger(
            Hertz::new(audio_float(module, 1)?).ok_or(AudioPatchError::InvalidParameter)?,
            audio_unit(module, 2)?,
            audio_unit(module, 3)?,
        )),
        ModuleKind::Multiply => Ok(AudioModule::Binary {
            op: BinaryOp::Multiply,
            a: audio_sample(module, 0)?,
            b: audio_sample(module, 1)?,
        }),
        ModuleKind::Add => Ok(AudioModule::Binary {
            op: BinaryOp::Add,
            a: audio_sample(module, 0)?,
            b: audio_sample(module, 1)?,
        }),
        ModuleKind::GreaterThan => Ok(AudioModule::Binary {
            op: BinaryOp::GreaterThan,
            a: audio_sample(module, 0)?,
            b: audio_sample(module, 1)?,
        }),
        ModuleKind::LessThan => Ok(AudioModule::Binary {
            op: BinaryOp::LessThan,
            a: audio_sample(module, 0)?,
            b: audio_sample(module, 1)?,
        }),
        ModuleKind::Switch => Ok(AudioModule::Switch {
            a: audio_sample(module, 1)?,
            b: audio_sample(module, 2)?,
        }),
        ModuleKind::Random => Ok(AudioModule::Random),
        ModuleKind::Sample => {
            let ModuleBody::Sample { samples, .. } = &module.body else {
                unreachable!();
            };
            Ok(AudioModule::Sample {
                samples: Arc::clone(samples),
            })
        }
        ModuleKind::Probe => Ok(AudioModule::Probe),
        ModuleKind::RightJoin | ModuleKind::DownJoin => Ok(AudioModule::Binary {
            op: BinaryOp::Add,
            a: AudioSample::ZERO,
            b: AudioSample::ZERO,
        }),
        ModuleKind::CompositionInput => {
            let ModuleBody::CompositionInput { value, .. } = &module.body else {
                unreachable!();
            };
            Ok(AudioModule::Binary {
                op: BinaryOp::Add,
                a: AudioSample::new(value.value as f32 / 100.0)
                    .ok_or(AudioPatchError::InvalidParameter)?,
                b: AudioSample::ZERO,
            })
        }
        ModuleKind::TurnRightDown
        | ModuleKind::TurnDownRight
        | ModuleKind::LeftSplit
        | ModuleKind::TopSplit
        | ModuleKind::CompositionOutput
        | ModuleKind::Composition => Ok(AudioModule::Pass),
        ModuleKind::Output => Ok(AudioModule::Pass),
    }
}

pub(super) fn audio_id(ids: &[AudioNode], key: AudioKey) -> Option<AudioModuleId> {
    ids.iter()
        .find_map(|candidate| (candidate.key == key).then_some(candidate.id))
}

pub(super) fn root_connection_source(
    state: &GuiState,
    root_modules: &[Module],
    ids: &[AudioNode],
    connection: &Connection,
) -> Option<(AudioModuleId, u16)> {
    let source = root_modules
        .iter()
        .find(|module| module.id == connection.from)?;
    if !source.is_composition() {
        return audio_id(ids, AudioKey::Root(source.id)).map(|id| (id, 0));
    }
    let surface = source.composition_surface()?;
    let terminal = composition_outputs(surface)
        .get(connection.output)
        .copied()?;
    let semantic = state.semantic_surface_connections(&surface.modules).ok()?;
    let output = semantic
        .iter()
        .find(|connection| connection.to == terminal)?;
    let module = surface
        .modules
        .iter()
        .find(|module| module.id == output.from)?;
    audio_id(
        ids,
        AudioKey::Composition {
            owner: source.id,
            module: output.from,
        },
    )
    .map(|id| {
        (
            id,
            module
                .is_composition()
                .then_some(output.output as u16)
                .unwrap_or(0),
        )
    })
}

fn composition_input_key(module: &Module) -> u16 {
    module.position.y
}

fn composition_output_key(module: &Module) -> u16 {
    module.position.x
}

pub(super) fn composition_inputs(surface: &PatchSurface) -> Vec<ModuleId> {
    let mut modules = surface
        .modules
        .iter()
        .filter(|module| !module.disabled && module.kind() == ModuleKind::CompositionInput)
        .collect::<Vec<_>>();
    modules.sort_by_key(|module| composition_input_key(module));
    modules.into_iter().map(|module| module.id).collect()
}

pub(super) fn composition_outputs(surface: &PatchSurface) -> Vec<ModuleId> {
    let mut modules = surface
        .modules
        .iter()
        .filter(|module| !module.disabled && module.kind() == ModuleKind::CompositionOutput)
        .collect::<Vec<_>>();
    modules.sort_by_key(|module| composition_output_key(module));
    modules.into_iter().map(|module| module.id).collect()
}

fn audio_wave(module: &Module) -> Result<Wave, AudioPatchError> {
    let Some(ParameterValue::Enum { index, .. }) = module.parameter(0).map(|p| p.value) else {
        return Err(AudioPatchError::InvalidParameter);
    };
    match index {
        0 => Ok(Wave::Sine),
        1 => Ok(Wave::Square),
        2 => Ok(Wave::Triangle),
        3 => Ok(Wave::Saw),
        4 => Ok(Wave::ReverseSaw),
        5 => Ok(Wave::Noise),
        _ => Err(AudioPatchError::InvalidParameter),
    }
}

fn audio_rate(
    module: &Module,
    parameter: usize,
    rate: SampleRate,
    bpm: u16,
) -> Result<f32, AudioPatchError> {
    let Some(value) = module.parameter(parameter).map(|parameter| parameter.value) else {
        return Err(AudioPatchError::InvalidParameter);
    };
    let value = match value {
        ParameterValue::Time {
            value,
            unit: TimeUnit::Hertz,
        } => value as f32,
        ParameterValue::Time {
            value,
            unit: TimeUnit::Seconds,
        } => 100.0 / value.max(1) as f32,
        ParameterValue::Time {
            value,
            unit: TimeUnit::Samples,
        } => rate.value() as f32 / value.max(1) as f32,
        ParameterValue::Bars {
            numerator,
            denominator,
        } => bpm.max(1) as f32 * denominator.max(1) as f32 / numerator.max(1) as f32 / 240.0,
        ParameterValue::Time {
            unit: TimeUnit::Bars,
            ..
        } => return Err(AudioPatchError::InvalidParameter),
        _ => return Err(AudioPatchError::InvalidParameter),
    };
    Hertz::new(value)
        .map(Hertz::value)
        .ok_or(AudioPatchError::InvalidParameter)
}

fn audio_duration(
    module: &Module,
    parameter: usize,
    rate: SampleRate,
    bpm: u16,
) -> Result<Duration, AudioPatchError> {
    let Some(value) = module.parameter(parameter).map(|p| p.value) else {
        return Err(AudioPatchError::InvalidParameter);
    };
    match value {
        ParameterValue::Time {
            value,
            unit: TimeUnit::Seconds,
        } => seconds(value as f32 / 100.0),
        ParameterValue::Time {
            value,
            unit: TimeUnit::Samples,
        } => Ok(Duration::Samples(Samples::new(value.max(1) as u64))),
        ParameterValue::Time {
            value,
            unit: TimeUnit::Bars,
        } => {
            let beats = value as f32 / 16.0 * 4.0;
            seconds(beats * 60.0 / bpm.max(1) as f32)
        }
        ParameterValue::Bars {
            numerator,
            denominator,
        } => {
            let beats = numerator.max(1) as f32 / denominator.max(1) as f32 * 4.0;
            seconds(beats * 60.0 / bpm.max(1) as f32)
        }
        ParameterValue::Time {
            value,
            unit: TimeUnit::Hertz,
        } => {
            let hz = value.max(1) as f32;
            seconds(1.0 / hz.min(rate.value() as f32))
        }
        _ => Err(AudioPatchError::InvalidParameter),
    }
}

fn seconds(value: f32) -> Result<Duration, AudioPatchError> {
    Seconds::new(value)
        .map(Duration::Seconds)
        .ok_or(AudioPatchError::InvalidParameter)
}

pub(super) fn audio_float(module: &Module, parameter: usize) -> Result<f32, AudioPatchError> {
    let Some(ParameterValue::Float { value, .. }) = module.parameter(parameter).map(|p| p.value)
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    Ok(value as f32 / 100.0)
}

fn audio_sample(module: &Module, parameter: usize) -> Result<AudioSample, AudioPatchError> {
    AudioSample::new(audio_float(module, parameter)?).ok_or(AudioPatchError::InvalidParameter)
}

fn audio_int(module: &Module, parameter: usize) -> Result<i32, AudioPatchError> {
    let Some(ParameterValue::Int { value, .. }) = module.parameter(parameter).map(|p| p.value)
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    Ok(value)
}

fn audio_unit(module: &Module, parameter: usize) -> Result<Unit, AudioPatchError> {
    Unit::new(audio_float(module, parameter)?.clamp(0.0, 1.0))
        .ok_or(AudioPatchError::InvalidParameter)
}

fn filter_cutoff(value: f32) -> Result<Hertz, AudioPatchError> {
    Hertz::new(20.0 * 1000.0_f32.powf(value.clamp(0.0, 1.0)))
        .ok_or(AudioPatchError::InvalidParameter)
}

fn audio_distortion(module: &Module) -> Result<AudioDistortion, AudioPatchError> {
    let Some(ParameterValue::Enum { index, .. }) = module.parameter(1).map(|p| p.value) else {
        return Err(AudioPatchError::InvalidParameter);
    };
    match index {
        0 => Ok(AudioDistortion::Tube),
        1 => Ok(AudioDistortion::Tape),
        2 => Ok(AudioDistortion::Fuzz),
        3 => Ok(AudioDistortion::Fold),
        4 => Ok(AudioDistortion::Clip),
        _ => Err(AudioPatchError::InvalidParameter),
    }
}

pub(super) fn scale_from_index(index: usize) -> Scale {
    match index {
        0 => chromatic(),
        1 => cmaj(),
        2 => cmin(),
        3 => csharpmaj(),
        4 => csharpmin(),
        5 => dmaj(),
        6 => dmin(),
        7 => dsharpmaj(),
        8 => dsharpmin(),
        9 => emaj(),
        10 => emin(),
        11 => fmaj(),
        12 => fmin(),
        13 => fsharpmaj(),
        14 => fsharpmin(),
        15 => gmaj(),
        16 => gmin(),
        17 => gsharpmaj(),
        18 => gsharpmin(),
        19 => amaj(),
        20 => amin(),
        21 => asharpmaj(),
        22 => asharpmin(),
        23 => bmaj(),
        24 => bmin(),
        _ => cmin(),
    }
}
