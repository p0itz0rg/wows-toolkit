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
        battle_controller::{BattleController, EventHandler},
        Analyzer, AnalyzerBuilder,
    },
    packet2::{Packet, PacketType, PacketTypeKind},
    parse_scripts,
    rpc::typedefs::ArgValue,
    ReplayFile, ReplayMeta,
};

use itertools::Itertools;

use crate::{
    app::{ReplayParserTabState, ToolkitTabViewer},
    game_params::GameMetadataProvider,
};

const CHAT_VIEW_WIDTH: f32 = 200.0;

pub type SharedReplayParserTabState = Arc<Mutex<ReplayParserTabState>>;

struct ReplayEventHandler {
    replay_tab_state: SharedReplayParserTabState,
}

impl ReplayEventHandler {
    pub fn new(game_meta: ReplayMeta, replay_tab_state: SharedReplayParserTabState) -> Self {
        Self { replay_tab_state }
    }
}

impl EventHandler for ReplayEventHandler {
    fn on_chat_message(&self, message: wows_replays::analyzer::battle_controller::GameMessage) {
        self.replay_tab_state
            .lock()
            .unwrap()
            .game_chat
            .push(message)
    }
}

#[self_referencing]
struct Replay<'res> {
    replay_file: ReplayFile,

    resource_loader: &'res GameMetadataProvider,

    #[borrows(replay_file, resource_loader)]
    #[not_covariant]
    battle_controller: BattleController<'this, 'this, GameMetadataProvider>,
}

impl ToolkitTabViewer<'_> {
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

                    let replay = ReplayBuilder {
                        replay_file,
                        resource_loader: &self.tab_state.game_metadata.unwrap(),
                        battle_controller_builder: |replay_file, resource_loader| {
                            BattleController::new(&replay_file.meta, resource_loader)
                        },
                    }
                    .build();

                    if let (Some(file_tree), Some(pkg_loader)) =
                        (&self.tab_state.file_tree, &self.tab_state.pkg_loader)
                    {
                        let data_file_loader =
                            wows_replays::version::DataFileWithCallback::new(|path| {
                                println!("requesting file: {path}");

                                let path = Path::new(path);

                                let mut file_data = Vec::new();
                                file_tree
                                    .read_file_at_path(path, pkg_loader, &mut file_data)
                                    .expect("failed to read file");

                                Ok(Cow::Owned(file_data))
                            });

                        let analyzer = Rc::new(ReplayEventHandler::new(
                            replay_file.meta.clone(),
                            Arc::clone(&self.tab_state.replay_parser_tab),
                        ));
                        let specs = parse_scripts(&data_file_loader).unwrap();

                        let version_parts: Vec<_> =
                            replay_file.meta.clientVersionFromExe.split(",").collect();
                        assert!(version_parts.len() == 4);

                        let processor = analyzer;

                        // Parse packets
                        let mut p = wows_replays::packet2::Parser::new(&specs);
                        let mut analyzer_set =
                            wows_replays::analyzer::AnalyzerAdapter::new(vec![Box::new(
                                battle_controller,
                            )]);

                        match p.parse_packets::<wows_replays::analyzer::AnalyzerAdapter>(
                            &replay_file.packet_data,
                            &mut analyzer_set,
                        ) {
                            Ok(()) => {
                                analyzer_set.finish();
                            }
                            Err(e) => panic!("{:?}", e),
                        }
                    }

                    self.tab_state.settings.current_replay = Some(replay_file);
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

            if let Some(replay_file) = &self.tab_state.settings.current_replay {
                let meta = &replay_file.meta;
                std::fs::write("meta.txt", format!("{:#?}", meta).as_bytes());
                ui.horizontal(|ui| {
                    ui.heading(meta.playerName.as_str());
                    ui.label(meta.matchGroup.as_str());
                    ui.label(meta.clientVersionFromExe.as_str());
                    if let Some(translations) = self.tab_state.translations.as_ref() {
                        let id = format!("IDS_{}", meta.scenario.to_uppercase());
                        ui.label(translations.gettext(id.as_str()));

                        let id = format!("IDS_{}", meta.mapName.to_uppercase());
                        ui.label(translations.gettext(id.as_str()));
                    } else {
                        ui.label(meta.scenario.as_str());
                        ui.label(meta.mapDisplayName.as_str());
                    }
                });

                StripBuilder::new(ui)
                    .size(Size::remainder())
                    .size(Size::exact(CHAT_VIEW_WIDTH))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
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
                                    let mut sorted_players = meta.vehicles.clone();
                                    sorted_players.sort_by(|a, b| a.relation.cmp(&b.relation));
                                    for player in &sorted_players {
                                        body.row(30.0, |mut ui| {
                                            ui.col(|ui| {
                                                ui.label(player.name.clone());
                                            });
                                            ui.col(|ui| {
                                                ui.label(match player.relation {
                                                    0 => "Self".to_string(),
                                                    1 => "Friendly".to_string(),
                                                    other => format!("Enemy Team ({other})"),
                                                });
                                            });
                                            ui.col(|ui| {
                                                ui.label(format!("{}", player.id));
                                            });
                                            ui.col(|ui| {
                                                if let (Some(game_params), Some(translations)) = (
                                                    &self.tab_state.game_metadata,
                                                    &self.tab_state.translations,
                                                ) {
                                                    let translation_id = game_params
                                                        .param_localization_id(player.shipId as u32)
                                                        .unwrap();
                                                    ui.label(translations.gettext(translation_id));
                                                } else {
                                                    ui.label(format!("{}", player.shipId));
                                                }
                                            });
                                            ui.col(|ui| {
                                                let mut got_ship_info = true;
                                                if let (Some(game_params), Some(translations)) = (
                                                    &self.tab_state.game_metadata,
                                                    &self.tab_state.translations,
                                                ) {
                                                    if let Some(vehicle) = game_params
                                                        .vehicle_by_id(player.shipId as u32)
                                                    {
                                                        let translation_id = vehicle
                                                            .species()
                                                            .map(|s| s.translation_id())
                                                            .expect("failed to get translation ID");
                                                        ui.label(
                                                            translations
                                                                .gettext(translation_id.as_str()),
                                                        );
                                                    } else {
                                                        println!("failed to get ship info");
                                                        got_ship_info = false;
                                                    }
                                                }
                                                if !got_ship_info {
                                                    ui.label("unk");
                                                }
                                            });

                                            ui.col(|ui| {
                                                ui.label("foo");
                                                // let replay_tab = self
                                                //     .tab_state
                                                //     .replay_parser_tab
                                                //     .lock()
                                                //     .unwrap();
                                                // let entity_id = replay_tab
                                                //     .vehicle_id_to_entity_id
                                                //     .get(&(player.id as u32))
                                                //     .expect("failed to get entity ID");

                                                // let ship_loadouts =
                                                //     replay_tab.ship_configs.get(entity_id).unwrap();
                                                // let skills = [
                                                //     &ship_loadouts.skills.aircraft_carrier,
                                                //     &ship_loadouts.skills.auxiliary,
                                                //     &ship_loadouts.skills.battleship,
                                                //     &ship_loadouts.skills.cruiser,
                                                //     &ship_loadouts.skills.destroyer,
                                                //     &ship_loadouts.skills.submarine,
                                                // ];
                                                // for skillset in skills {
                                                //     if !skillset.is_empty() {
                                                //         ui.label(format!("{}", skillset.len()));
                                                //         break;
                                                //     }
                                                // }
                                            });
                                        });
                                    }
                                });
                        });
                        strip.cell(|ui| {
                            let tab_state = self.tab_state.replay_parser_tab.lock().unwrap();
                            egui::ScrollArea::vertical()
                                .id_source("game_chat_scroll_area")
                                .show(ui, |ui| {
                                    egui::Grid::new("filtered_files_grid")
                                        .max_col_width(CHAT_VIEW_WIDTH)
                                        .num_columns(1)
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for message in &tab_state.game_chat {
                                                let GameMessage {
                                                    sender_relation,
                                                    sender_name,
                                                    channel,
                                                    message,
                                                } = message;

                                                let text = format!(
                                                    "{sender_name} ({channel:?}): {message}"
                                                );

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
                                                    ui.output_mut(|output| {
                                                        output.copied_text = text
                                                    });
                                                }
                                                ui.end_row();
                                            }
                                        });
                                });
                        });
                    });
            }
        });
    }
}
