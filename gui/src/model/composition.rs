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

fn composition_input_position(module: &Module, input: usize) -> Option<(u16, u16)> {
    if module.kind() == ModuleKind::Delay {
        return match module.orientation {
            Orientation::Right => match input {
                0 => Some((module.position.x, module.position.y)),
                1 => Some((module.position.x, module.position.y)),
                2 => Some((module.position.x, module.position.y + 1)),
                _ => None,
            },
            Orientation::Down => match input {
                0 => Some((module.position.x, module.position.y)),
                1 => Some((module.position.x, module.position.y)),
                2 => Some((module.position.x + 1, module.position.y)),
                _ => None,
            },
        };
    }
    Some(match module.orientation {
        Orientation::Down => (module.position.x + input as u16, module.position.y),
        Orientation::Right => (module.position.x, module.position.y + input as u16),
    })
}

pub(super) fn composition_body(
    graph: Box<brainwash::patch::Composition>,
    next_module_id: &mut u32,
) -> Option<ModuleBody> {
    const FDN_POSITIONS: [(u16, u16); 58] = [
        (9, 5),
        (9, 6),
        (9, 7),
        (9, 8),
        (20, 0),
        (20, 10),
        (22, 1),
        (22, 3),
        (22, 5),
        (22, 7),
        (22, 11),
        (22, 13),
        (22, 15),
        (22, 17),
        (18, 0),
        (18, 1),
        (18, 2),
        (18, 3),
        (18, 10),
        (18, 11),
        (18, 12),
        (18, 13),
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 1),
        (5, 1),
        (6, 1),
        (7, 1),
        (0, 11),
        (1, 11),
        (2, 11),
        (3, 11),
        (4, 11),
        (5, 11),
        (6, 11),
        (7, 11),
        (0, 6),
        (1, 6),
        (2, 6),
        (3, 6),
        (4, 6),
        (5, 6),
        (6, 6),
        (7, 6),
        (28, 1),
        (28, 2),
        (28, 3),
        (28, 4),
        (28, 5),
        (28, 6),
        (28, 7),
        (28, 8),
        (0, 3),
        (0, 13),
        (0, 8),
        (29, 1),
    ];
    const FDN_VOICE_GROUP_POSITIONS: [(u16, u16); 11] = [
        (0, 0),
        (2, 1),
        (4, 2),
        (6, 3),
        (8, 8),
        (10, 7),
        (12, 6),
        (14, 5),
        (16, 4),
        (18, 8),
        (20, 0),
    ];
    const REVERB_POSITIONS: [(u16, u16); 7] =
        [(0, 0), (3, 2), (3, 3), (3, 4), (0, 1), (2, 0), (4, 1)];
    const FEEDBACK_GROUP_POSITIONS: [(u16, u16); 12] = [
        (0, 0),
        (2, 0),
        (4, 0),
        (6, 0),
        (8, 0),
        (10, 0),
        (12, 0),
        (14, 0),
        (16, 1),
        (18, 6),
        (20, 11),
        (22, 16),
    ];
    const REFLECTION_POSITIONS: [(u16, u16); 16] = [
        (0, 0),
        (2, 0),
        (4, 0),
        (6, 0),
        (8, 0),
        (10, 0),
        (12, 0),
        (14, 0),
        (16, 1),
        (18, 3),
        (20, 5),
        (22, 7),
        (24, 9),
        (26, 11),
        (28, 13),
        (30, 15),
    ];
    const OUTPUT_DECODER_POSITIONS: [(u16, u16); 13] = [
        (0, 0),
        (2, 0),
        (4, 0),
        (6, 0),
        (8, 0),
        (10, 0),
        (12, 0),
        (14, 0),
        (16, 0),
        (18, 1),
        (20, 6),
        (22, 11),
        (24, 13),
    ];
    let entries = graph
        .patch()
        .module_entries()
        .map(|(id, module)| (id, module.clone()))
        .collect::<Vec<_>>();
    let connections = graph.patch().connection_entries().collect::<Vec<_>>();
    let mut remaining = entries.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let index = remaining
            .iter()
            .position(|candidate| {
                !connections.iter().any(|(from, input)| {
                    input.module() == *candidate && remaining.contains(&from.module())
                })
            })
            .expect("composition is acyclic");
        ordered.push(remaining.remove(index));
    }
    let mut modules = Vec::new();
    let ids = entries
        .iter()
        .enumerate()
        .map(|(index, (id, _))| (*id, ModuleId::new(*next_module_id + index as u32)))
        .collect::<Vec<_>>();
    *next_module_id += ids.len() as u32;
    let linear = graph.outputs().len() == 1
        && entries.iter().all(|(id, _)| {
            connections
                .iter()
                .filter(|(from, _)| from.module() == *id)
                .count()
                <= 1
                && connections
                    .iter()
                    .filter(|(_, input)| input.module() == *id)
                    .count()
                    <= 1
        });
    let mut positions = HashMap::new();
    let mut fallback_output_y = 0;
    if linear {
        for core_id in &ordered {
            let node = entries
                .iter()
                .find_map(|(id, module)| (*id == *core_id).then_some(module))
                .expect("composition module exists");
            let incoming = connections
                .iter()
                .find(|(_, input)| input.module() == *core_id);
            let x = incoming
                .and_then(|(from, _)| positions.get(&from.module()))
                .map_or(0, |position: &GridPos| position.x + 1);
            let y = incoming
                .and_then(|(from, input)| {
                    let source = entries
                        .iter()
                        .find_map(|(id, module)| (*id == from.module()).then_some(module))?;
                    let source_position = positions.get(&from.module())?;
                    let source_height = source
                        .input_kinds()
                        .len()
                        .max(source.output_count() as usize)
                        .max(1) as u16;
                    let source_y =
                        source_position.y + source_height - source.output_count() + from.index();
                    let input_index =
                        node.input_kinds()
                            .iter()
                            .position(|kind| *kind == input.kind())? as u16;
                    source_y.checked_sub(input_index)
                })
                .unwrap_or(0);
            positions.insert(*core_id, GridPos::new(x, y));
        }
    } else {
        let mut x = 0;
        let mut y = 0;
        for core_id in &ordered {
            let node = entries
                .iter()
                .find_map(|(id, module)| (*id == *core_id).then_some(module))
                .expect("composition module exists");
            positions.insert(*core_id, GridPos::new(x, y));
            x += 2;
            y += (node.input_kinds().len() as u16).max(
                matches!(node, AudioModule::Input { .. })
                    .then_some(u16::from(
                        graph.inputs().len() <= 5 && graph.name() != "ADSR",
                    ))
                    .or_else(|| {
                        graph
                            .outputs()
                            .iter()
                            .any(|output| output.port().module() == *core_id)
                            .then_some(1)
                    })
                    .or_else(|| (node.output_count() > 1).then_some(node.output_count()))
                    .unwrap_or(0),
            );
        }
        fallback_output_y = y;
    }
    for (core_id, node) in &entries {
        let projected = ids
            .iter()
            .find_map(|(id, projected)| (*id == *core_id).then_some(*projected))
            .expect("projected module exists");
        let body = if let Some(input) = graph
            .inputs()
            .iter()
            .find(|input| input.module() == *core_id)
        {
            let default = match node {
                AudioModule::Input { default, .. } => default.value(),
                _ => 0.0,
            };
            ModuleBody::CompositionInput {
                label: input.label().to_string(),
                kind: input.kind(),
                value: float_param(-100_000, 100_000, 1, (default * 100.0).round() as i32),
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
                    gain.value = (tap.gain().value() * 100.0).round() as i32;
                    body
                }
                _ => graph_node_body(node),
            }
        };
        let position = if graph.name() == "FDN Tank" && entries.len() == FDN_POSITIONS.len() {
            let index = entries.iter().position(|(id, _)| *id == *core_id)?;
            let (x, y) = FDN_POSITIONS[index];
            GridPos::new(x, y)
        } else if graph.name() == "Reverb" && entries.len() == REVERB_POSITIONS.len() {
            let index = entries.iter().position(|(id, _)| *id == *core_id)?;
            let (x, y) = REVERB_POSITIONS[index];
            GridPos::new(x, y)
        } else if graph.name() == "FDN Voice Group"
            && entries.len() == FDN_VOICE_GROUP_POSITIONS.len()
        {
            let index = entries.iter().position(|(id, _)| *id == *core_id)?;
            let (x, y) = FDN_VOICE_GROUP_POSITIONS[index];
            GridPos::new(x, y)
        } else if graph.name() == "Feedback Group"
            && entries.len() == FEEDBACK_GROUP_POSITIONS.len()
        {
            let index = entries.iter().position(|(id, _)| *id == *core_id)?;
            let (x, y) = FEEDBACK_GROUP_POSITIONS[index];
            GridPos::new(x, y)
        } else if graph.name() == "Reflection" && entries.len() == REFLECTION_POSITIONS.len() {
            let index = entries.iter().position(|(id, _)| *id == *core_id)?;
            let (x, y) = REFLECTION_POSITIONS[index];
            GridPos::new(x, y)
        } else if matches!(graph.name(), "Output Decoder" | "Reflection") {
            let index = entries.iter().position(|(id, _)| *id == *core_id)?;
            let (x, y) = OUTPUT_DECODER_POSITIONS[index];
            GridPos::new(x, y)
        } else {
            *positions.get(core_id)?
        };
        modules.push(Module {
            id: projected,
            position,
            orientation: if graph.name() == "FDN Tank"
                && matches!(
                    entries.iter().position(|(id, _)| *id == *core_id),
                    Some(0..=21 | 46..=53 | 57)
                ) {
                Orientation::Right
            } else if graph.name() == "FDN Tank" {
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
    for (index, declared) in graph.outputs().iter().enumerate() {
        let source = ids
            .iter()
            .find_map(|(id, projected)| (declared.port().module() == *id).then_some(*projected))?;
        let source_module = modules.iter().find(|module| module.id == source)?;
        let position = if graph.name() == "FDN Tank" && entries.len() == FDN_POSITIONS.len() {
            GridPos::new(30, 10 + index as u16)
        } else if graph.name() == "Output Decoder"
            && entries.len() == OUTPUT_DECODER_POSITIONS.len()
        {
            GridPos::new(31, 17 + index as u16)
        } else if graph.outputs().len() == 1 && linear {
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
                source_module.position.x + module_footprint(source_module, source_ports).0,
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
            orientation: if graph.outputs().len() == 1
                && graph.name() != "Output Decoder"
                && !linear
            {
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
    if graph.name() == "FDN Tank" && entries.len() == FDN_POSITIONS.len() {
        modules.push(Module {
            id: ModuleId::new(*next_module_id),
            position: GridPos::new(30, 9),
            orientation: Orientation::Right,
            body: ModuleBody::TurnRightDown,
            disabled: false,
        });
        *next_module_id += 1;
        let mut occupied = HashSet::new();
        for module in &modules {
            let (width, height) =
                module_footprint(module, module.composition_surface().map(composition_ports));
            for y in module.position.y..module.position.y + height {
                for x in module.position.x..module.position.x + width {
                    occupied.insert(GridPos::new(x, y));
                }
            }
        }
        for (core_source, projected_source) in &ids {
            let source = modules
                .iter()
                .find(|module| module.id == *projected_source)?;
            let source_position = source.position;
            let source_orientation = source.orientation;
            let (source_width, source_height) =
                module_footprint(source, source.composition_surface().map(composition_ports));
            let core_module = entries
                .iter()
                .find_map(|(id, module)| (*id == *core_source).then_some(module))?;
            for source_output in 0..core_module.output_count() {
                if graph.patch().output_module().is_some_and(|output| {
                    output.module() == *core_source && output.index() == source_output
                }) {
                    continue;
                }
                let (source_x, source_y) = match source_orientation {
                    Orientation::Down => (
                        source_position.x + source_width - core_module.output_count()
                            + source_output,
                        source_position.y,
                    ),
                    Orientation::Right => (
                        source_position.x,
                        source_position.y + source_height - core_module.output_count()
                            + source_output,
                    ),
                };
                let mut targets = connections
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
                        composition_input_position(target_module, input_index)
                    })
                    .collect::<Vec<_>>();
                targets.sort_unstable();
                targets.dedup();
                if targets.is_empty() {
                    continue;
                }
                if source_orientation == Orientation::Right
                    && targets.iter().all(|target| target.0 == targets[0].0)
                    && targets[0].1 == source_y
                {
                    if targets.len() > 1 {
                        let branch_x = (source_x + 1..targets[0].0).rev().find(|x| {
                            (source_y..=targets.last().unwrap().1)
                                .all(|y| !occupied.contains(&GridPos::new(*x, y)))
                        });
                        let branch_x = branch_x?;
                        for (index, (_, target_y)) in targets.iter().enumerate() {
                            modules.push(Module {
                                id: ModuleId::new(*next_module_id),
                                position: GridPos::new(branch_x, *target_y),
                                orientation: Orientation::Right,
                                body: if index + 1 == targets.len() {
                                    ModuleBody::TurnDownRight
                                } else {
                                    ModuleBody::LeftSplit
                                },
                                disabled: false,
                            });
                            occupied.insert(GridPos::new(branch_x, *target_y));
                            *next_module_id += 1;
                        }
                    }
                    continue;
                }
                if targets.len() == 1
                    && targets[0].1 == source_y
                    && (source_x + 1..targets[0].0)
                        .all(|x| !occupied.contains(&GridPos::new(x, source_y)))
                {
                    continue;
                }
                if targets.len() == 1
                    && targets[0].1 == source_y + 1
                    && targets[0].0 > source_x
                    && (source_x + 1..targets[0].0)
                        .all(|x| !occupied.contains(&GridPos::new(x, targets[0].1)))
                {
                    modules.push(Module {
                        id: ModuleId::new(*next_module_id),
                        position: GridPos::new(source_x, targets[0].1),
                        orientation: Orientation::Right,
                        body: ModuleBody::TurnDownRight,
                        disabled: false,
                    });
                    occupied.insert(GridPos::new(source_x, targets[0].1));
                    *next_module_id += 1;
                    continue;
                }
                if targets.len() == 1
                    && targets[0].0 == source_x
                    && (source_y + 1..targets[0].1)
                        .all(|y| !occupied.contains(&GridPos::new(source_x, y)))
                {
                    continue;
                }
                let last = targets.last()?.0;
                let max_bus_y = targets.iter().map(|(_, y)| *y).min()?.checked_sub(1)?;
                let bus_y = (source_y + 1..=max_bus_y).find(|bus_y| {
                    (source_y + 1..=*bus_y).all(|y| !occupied.contains(&GridPos::new(source_x, y)))
                        && (source_x..=last).all(|x| !occupied.contains(&GridPos::new(x, *bus_y)))
                        && targets.iter().all(|(x, target_y)| {
                            (*bus_y..*target_y).all(|y| !occupied.contains(&GridPos::new(*x, y)))
                        })
                });
                let Some(bus_y) = bus_y else {
                    return None;
                };
                let same_column = targets
                    .first()
                    .is_some_and(|(column, _)| *column == source_x);
                modules.push(Module {
                    id: ModuleId::new(*next_module_id),
                    position: GridPos::new(source_x, bus_y),
                    orientation: Orientation::Right,
                    body: if same_column {
                        ModuleBody::TopSplit
                    } else {
                        ModuleBody::TurnDownRight
                    },
                    disabled: false,
                });
                occupied.insert(GridPos::new(source_x, bus_y));
                *next_module_id += 1;
                let mut columns = targets
                    .into_iter()
                    .map(|(column, _)| column)
                    .filter(|column| *column > source_x)
                    .collect::<Vec<_>>();
                columns.dedup();
                for column in columns {
                    modules.push(Module {
                        id: ModuleId::new(*next_module_id),
                        position: GridPos::new(column, bus_y),
                        orientation: Orientation::Right,
                        body: if column == last {
                            ModuleBody::TurnRightDown
                        } else {
                            ModuleBody::LeftSplit
                        },
                        disabled: false,
                    });
                    occupied.insert(GridPos::new(column, bus_y));
                    *next_module_id += 1;
                }
            }
        }
        return Some(ModuleBody::Composition {
            name: graph.name().to_string(),
            surface: PatchSurface {
                cursor: GridPos::new(0, 0),
                modules,
            },
        });
    }
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
                .filter(|(from, _)| from.module() == *core_source && from.index() == source_output)
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
                    composition_input_position(target_module, input_index).map(|(_, y)| y)
                })
                .collect::<Vec<_>>();
            let output_positions = exposed_outputs
                .iter()
                .filter_map(|(port, position)| {
                    (port.module() == *core_source && port.index() == source_output)
                        .then_some(*position)
                })
                .collect::<Vec<_>>();
            let bus_x = source_x + source_width;
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
    Some(ModuleBody::Composition {
        name: graph.name().to_string(),
        surface: PatchSurface {
            cursor: GridPos::new(0, 0),
            modules,
        },
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
