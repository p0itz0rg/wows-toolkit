use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, Cursor},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};

use bounded_vec_deque::BoundedVecDeque;
use byteorder::{LittleEndian, ReadBytesExt};
use egui::{text::LayoutJob, Color32, Label, Sense, Separator, TextFormat};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use ouroboros::self_referencing;
use serde::{Deserialize, Serialize};
use wows_replays::{
    analyzer::{
        battle_controller::{
            self, BattleController, BattleReport, ChatChannel, EventHandler, GameMessage,
        },
        AnalyzerBuilder, AnalyzerMut,
    },
    packet2::{Packet, PacketType, PacketTypeKind},
    parse_scripts,
    resource_loader::ResourceLoader,
    rpc::typedefs::ArgValue,
    ReplayFile, ReplayMeta,
};

use itertools::Itertools;
use wowsunpack::{idx::FileNode, pkg::PkgFileLoader};

use crate::{
    app::{ReplayParserTabState, ToolkitTabViewer},
    game_params::GameMetadataProvider,
};

const CHAT_VIEW_WIDTH: f32 = 200.0;

pub type SharedReplayParserTabState = Arc<Mutex<ReplayParserTabState>>;

pub struct Replay {
    replay_file: ReplayFile,

    resource_loader: Rc<GameMetadataProvider>,

    battle_report: Option<BattleReport>,
}

impl Replay {
    pub fn parse(&mut self, file_tree: &FileNode, pkg_loader: Arc<PkgFileLoader>) {
        let version_parts: Vec<_> = self
            .replay_file
            .meta
            .clientVersionFromExe
            .split(",")
            .collect();
        assert!(version_parts.len() == 4);

        // Parse packets
        let packet_data = &self.replay_file.packet_data;
        let mut controller =
            BattleController::new(&self.replay_file.meta, self.resource_loader.as_ref());
        let mut p = wows_replays::packet2::Parser::new(&self.resource_loader.entity_specs());

        match p.parse_packets_mut(packet_data, &mut controller) {
            Ok(()) => {
                controller.finish();
                self.battle_report = Some(controller.build_report());
            }
            Err(e) => panic!("{:?}", e),
        }
    }
}

impl ToolkitTabViewer<'_> {
    fn build_replay_player_list(&self, report: &BattleReport, ui: &mut egui::Ui) {
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::initial(100.0).range(40.0..=300.0))
            .column(Column::initial(100.0).at_least(40.0).clip(true))
            .column(Column::initial(100.0).at_least(40.0).clip(true))
            .column(Column::initial(100.0).at_least(40.0).clip(true))
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Player Name");
                });
                header.col(|ui| {
                    ui.strong("Relation");
                });
                header.col(|ui| {
                    ui.strong("ID");
                });
                header.col(|ui| {
                    ui.strong("Ship Name");
                });
                header.col(|ui| {
                    ui.strong("Ship Class");
                });
                header.col(|ui| {
                    ui.strong("Allocated Skills");
                });
            })
            .body(|mut body| {
                let mut sorted_players = report.player_entities().to_vec();
                sorted_players.sort_by(|a, b| {
                    a.player()
                        .unwrap()
                        .relation()
                        .cmp(&b.player().unwrap().relation())
                });
                for entity in &sorted_players {
                    let player = entity.player().unwrap();
                    let ship = player.vehicle();
                    body.row(30.0, |mut ui| {
                        ui.col(|ui| {
                            ui.label(player.name());
                        });
                        ui.col(|ui| {
                            ui.label(match player.relation() {
                                0 => "Self".to_string(),
                                1 => "Friendly".to_string(),
                                other => {
                                    format!("Enemy Team ({other})")
                                }
                            });
                        });
                        ui.col(|ui| {
                            ui.label(format!("{}", player.avatar_id()));
                        });
                        ui.col(|ui| {
                            let ship_name = self
                                .tab_state
                                .world_of_warships_data
                                .game_metadata
                                .as_ref()
                                .unwrap()
                                .localized_name_from_param(ship)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| format!("{}", ship.id()));
                            ui.label(ship_name);
                        });
                        ui.col(|ui| {
                            let species: String = ship
                                .species()
                                .and_then(|species| {
                                    let species: &'static str = species.into();
                                    let id = format!("IDS_{}", species.to_uppercase());
                                    self.tab_state
                                        .world_of_warships_data
                                        .game_metadata
                                        .as_ref()
                                        .unwrap()
                                        .localized_name_from_id(&id)
                                })
                                .unwrap_or_else(|| "unk".to_string());
                            ui.label(species);
                        });

                        let captain = entity
                            .captain()
                            .data()
                            .crew_ref()
                            .expect("captain is not a crew?");
                        let species = ship.species().expect("ship has no species?");
                        let skill_points =
                            entity
                                .commander_skills()
                                .iter()
                                .fold(0usize, |accum, skill_type| {
                                    accum
                                        + captain
                                            .skill_by_type(*skill_type as u32)
                                            .expect("could not get skill type")
                                            .tier()
                                            .get_for_species(species.clone())
                                });

                        ui.col(|ui| {
                            ui.label(format!(
                                "{}pts ({} skills)",
                                skill_points,
                                entity.commander_skills().len()
                            ));
                        });
                    });
                }
            });
    }

    fn build_replay_chat(&self, battle_report: &BattleReport, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("game_chat_scroll_area")
            .show(ui, |ui| {
                egui::Grid::new("filtered_files_grid")
                    .max_col_width(CHAT_VIEW_WIDTH)
                    .num_columns(1)
                    .striped(true)
                    .show(ui, |ui| {
                        for message in battle_report.game_chat() {
                            let GameMessage {
                                sender_relation,
                                sender_name,
                                channel,
                                message,
                            } = message;

                            let text = format!("{sender_name} ({channel:?}): {message}");

                            let is_dark_mode = ui.visuals().dark_mode;
                            let name_color = match *sender_relation {
                                0 => Color32::GOLD,
                                1 => {
                                    if is_dark_mode {
                                        Color32::LIGHT_GREEN
                                    } else {
                                        Color32::DARK_GREEN
                                    }
                                }
                                _ => {
                                    if is_dark_mode {
                                        Color32::LIGHT_RED
                                    } else {
                                        Color32::DARK_RED
                                    }
                                }
                            };

                            let mut job = LayoutJob::default();
                            job.append(
                                &format!("{sender_name}: "),
                                0.0,
                                TextFormat {
                                    color: name_color,
                                    ..Default::default()
                                },
                            );

                            let text_color = match channel {
                                ChatChannel::Division => Color32::GOLD,
                                ChatChannel::Global => {
                                    if is_dark_mode {
                                        Color32::WHITE
                                    } else {
                                        Color32::BLACK
                                    }
                                }
                                ChatChannel::Team => {
                                    if is_dark_mode {
                                        Color32::LIGHT_GREEN
                                    } else {
                                        Color32::DARK_GREEN
                                    }
                                }
                            };

                            job.append(
                                message,
                                0.0,
                                TextFormat {
                                    color: text_color,
                                    ..Default::default()
                                },
                            );

                            if ui
                                .add(Label::new(job).sense(Sense::click()))
                                .on_hover_text(format!("Click to copy"))
                                .clicked()
                            {
                                ui.output_mut(|output| output.copied_text = text);
                            }
                            ui.end_row();
                        }
                    });
            });
    }

    fn build_replay_view(&self, replay_file: &Replay, ui: &mut egui::Ui) {
        if let Some(report) = replay_file.battle_report.as_ref() {
            ui.horizontal(|ui| {
                ui.heading(report.self_entity().player().unwrap().name());
                ui.label(report.match_group());
                ui.label(report.version().to_path());
                ui.label(report.game_mode());
                ui.label(report.map_name());
            });

            StripBuilder::new(ui)
                .size(Size::remainder())
                .size(Size::exact(CHAT_VIEW_WIDTH))
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        self.build_replay_player_list(report, ui);
                    });
                    strip.cell(|ui| {
                        self.build_replay_chat(report, ui);
                    });
                });
        }
    }

    /// Builds the replay parser tab
    pub fn build_replay_parser_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(
                        &mut self
                            .tab_state
                            .settings
                            .current_replay_path
                            .to_string_lossy()
                            .to_owned(),
                    )
                    .hint_text("Current Replay File"),
                );

                if ui.button("reparse").clicked() {
                    {
                        self.tab_state
                            .replay_parser_tab
                            .lock()
                            .unwrap()
                            .game_chat
                            .clear();
                    }
                    let replay_file: ReplayFile =
                        ReplayFile::from_file(&self.tab_state.settings.current_replay_path)
                            .unwrap();

                    let mut replay = Replay {
                        replay_file,
                        resource_loader: self
                            .tab_state
                            .world_of_warships_data
                            .game_metadata
                            .clone()
                            .unwrap(),
                        battle_report: None,
                    };

                    if let (Some(file_tree), Some(pkg_loader)) = (
                        self.tab_state.world_of_warships_data.file_tree.as_ref(),
                        self.tab_state.world_of_warships_data.pkg_loader.as_ref(),
                    ) {
                        replay.parse(file_tree, pkg_loader.clone());
                    }

                    self.tab_state.world_of_warships_data.current_replay = Some(replay);
                }

                if ui.button("parse").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("WoWs Replays", &["wowsreplay"])
                        .pick_file()
                    {
                        //println!("{:#?}", ReplayFile::from_file(&file));

                        self.tab_state.settings.current_replay_path = file;
                    }
                }
            });

            if let Some(replay_file) = self
                .tab_state
                .world_of_warships_data
                .current_replay
                .as_ref()
            {
                self.build_replay_view(replay_file, ui);
            }
        });
    }
}
