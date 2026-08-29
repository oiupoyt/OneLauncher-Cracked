use freya::prelude::*;
use oneclient_auth::MinecraftAccount;

use crate::components::{Button, Icon, IconType, OverlayPopup, PlayerModel, TextInput};
use crate::hooks::{
    delete_account_skin, fetch_skin_online, save_account_skin, try_accounts, try_default_account,
    use_accounts, use_current_account, use_custom_skin,
};
use crate::theme::colors;
use crate::ui::border_all_color;

#[derive(PartialEq)]
pub struct AccountSkins;

impl Component for AccountSkins {
    fn render(&self) -> impl IntoElement {
        let current_account = use_current_account();
        let default_account = try_default_account(&current_account);
        let accounts = try_accounts(&use_accounts()).unwrap_or_default();

        let selected_id = use_state(|| default_account.as_ref().map(|a| a.id));

        // Keep selected account in sync if none is explicitly picked
        let active_account = selected_id
            .read()
            .and_then(|id| accounts.iter().find(|a| a.id == id).cloned())
            .or_else(|| default_account.clone());

        let status_msg = use_state(|| None::<(String, bool)>); // (message, is_error)
        let show_url_modal = use_state(|| false);
        let is_loading = use_state(|| false);

        rect()
            .horizontal()
            .width(Size::fill())
            .height(Size::fill())
            .overflow(Overflow::Clip)
            .padding(40.)
            .spacing(28.)
            .child(preview_panel(active_account.clone(), status_msg))
            .child(side_panel(
                active_account,
                accounts,
                selected_id,
                status_msg,
                show_url_modal,
                is_loading,
            ))
            .maybe_child((*show_url_modal.read()).then(|| {
                url_modal(selected_id, status_msg, show_url_modal, is_loading)
            }))
    }
}

fn preview_panel(
    account: Option<MinecraftAccount>,
    status_msg: State<Option<(String, bool)>>,
) -> impl IntoElement {
    let account_uuid = account.as_ref().map(|a| a.id.to_string());
    let custom_query = use_custom_skin(account_uuid.clone().unwrap_or_default());
    let custom_data = crate::hooks::settled_or_loading(&custom_query);
    let is_slim = custom_data.as_ref().map_or(false, |d| d.is_slim);
    let has_custom = custom_data.as_ref().map_or(false, |d| d.has_custom);

    let acc_id_copy = account.as_ref().map(|a| a.id);
    let username_copy = account.as_ref().map(|a| a.username.clone());

    let set_classic = {
        let acc_id = acc_id_copy;
        let uname = username_copy.clone();
        let mut status = status_msg;
        move |_| {
            let Some(id) = acc_id else { return };
            let uname = uname.clone();
            let uuid_str = id.to_string();
            spawn(async move {
                if let Ok(skin_path) = oneclient_common::paths::skin_file_path(&uuid_str) {
                    if let Ok(bytes) = polyio::read(&skin_path).await {
                        let _ = save_account_skin(&uuid_str, uname.as_deref(), &bytes, false).await;
                        status.set(Some((
                            "Model set to Classic (4px arms)".to_string(),
                            false,
                        )));
                    } else if let Some(steve_bytes) = crate::AppAssets::get_bytes("steve.png") {
                        let _ = save_account_skin(&uuid_str, uname.as_deref(), &steve_bytes, false)
                            .await;
                        status.set(Some(("Model set to Classic".to_string(), false)));
                    }
                }
            });
        }
    };

    let set_slim = {
        let acc_id = acc_id_copy;
        let uname = username_copy.clone();
        let mut status = status_msg;
        move |_| {
            let Some(id) = acc_id else { return };
            let uname = uname.clone();
            let uuid_str = id.to_string();
            spawn(async move {
                if let Ok(skin_path) = oneclient_common::paths::skin_file_path(&uuid_str) {
                    if let Ok(bytes) = polyio::read(&skin_path).await {
                        let _ = save_account_skin(&uuid_str, uname.as_deref(), &bytes, true).await;
                        status.set(Some((
                            "Model set to Slim (3px arms)".to_string(),
                            false,
                        )));
                    } else if let Some(alex_bytes) = crate::AppAssets::get_bytes("alex.png") {
                        let _ = save_account_skin(&uuid_str, uname.as_deref(), &alex_bytes, true)
                            .await;
                        status.set(Some(("Model set to Slim".to_string(), false)));
                    }
                }
            });
        }
    };

    rect()
        .vertical()
        .width(Size::px(340.))
        .height(Size::fill())
        .spacing(16.)
        .child(
            rect()
                .center()
                .width(Size::fill())
                .height(Size::flex(1.0))
                .corner_radius(CornerRadius::new_all(16.))
                .background(colors::page_elevated())
                .border(border_all_color(1., colors::component_border()))
                .child(match account_uuid {
                    Some(uuid) => PlayerModel::new(uuid)
                        .width(Size::fill())
                        .height(Size::fill())
                        .into_element(),
                    None => Icon::new(IconType::Users01)
                        .size(64.)
                        .color(colors::fg_secondary())
                        .into_element(),
                }),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .main_align(Alignment::Center)
                .spacing(8.)
                .child(
                    rect()
                        .center()
                        .padding(Gaps::new_symmetric(6., 14.))
                        .corner_radius(CornerRadius::new_all(8.))
                        .background(if !is_slim {
                            colors::brand()
                        } else {
                            colors::component_bg()
                        })
                        .cursor(CursorIcon::Pointer)
                        .on_press(set_classic)
                        .child(
                            label()
                                .text("Classic (4px)")
                                .font_size(12.)
                                .font_weight(if !is_slim {
                                    FontWeight::SEMI_BOLD
                                } else {
                                    FontWeight::NORMAL
                                })
                                .color(colors::fg_primary()),
                        ),
                )
                .child(
                    rect()
                        .center()
                        .padding(Gaps::new_symmetric(6., 14.))
                        .corner_radius(CornerRadius::new_all(8.))
                        .background(if is_slim {
                            colors::brand()
                        } else {
                            colors::component_bg()
                        })
                        .cursor(CursorIcon::Pointer)
                        .on_press(set_slim)
                        .child(
                            label()
                                .text("Slim (3px)")
                                .font_size(12.)
                                .font_weight(if is_slim {
                                    FontWeight::SEMI_BOLD
                                } else {
                                    FontWeight::NORMAL
                                })
                                .color(colors::fg_primary()),
                        ),
                ),
        )
        .maybe_child(has_custom.then(|| {
            rect()
                .center()
                .child(
                    label()
                        .text("Custom Skin Active")
                        .font_size(11.)
                        .color(colors::success()),
                )
                .into_element()
        }))
        .into_element()
}

fn side_panel(
    active_account: Option<MinecraftAccount>,
    accounts: Vec<MinecraftAccount>,
    selected_id: State<Option<uuid::Uuid>>,
    status_msg: State<Option<(String, bool)>>,
    show_url_modal: State<bool>,
    is_loading: State<bool>,
) -> impl IntoElement {
    let username = active_account
        .as_ref()
        .map(|a| a.username.clone())
        .unwrap_or_else(|| "No Account Selected".to_string());
    let account_id = active_account.as_ref().map(|a| a.id);
    let username_opt = active_account.as_ref().map(|a| a.username.clone());

    // Import from local file
    let id_for_file = account_id;
    let uname_for_file = username_opt.clone();
    let mut status_file = status_msg;
    let mut loading_file = is_loading;

    let import_from_file = move |_| {
        let Some(id) = id_for_file else { return };
        let uname = uname_for_file.clone();
        let uuid_str = id.to_string();

        loading_file.set(true);
        spawn(async move {
            if let Some(handle) = rfd::AsyncFileDialog::new()
                .set_title("Select Minecraft Skin PNG")
                .add_filter("Minecraft Skin (*.png)", &["png"])
                .pick_file()
                .await
            {
                let bytes = handle.read().await;
                match save_account_skin(&uuid_str, uname.as_deref(), &bytes, false).await {
                    Ok(()) => {
                        status_file.set(Some((
                            "Custom skin imported successfully!".to_string(),
                            false,
                        )));
                    }
                    Err(err) => {
                        status_file.set(Some((err, true)));
                    }
                }
            }
            loading_file.set(false);
        });
    };

    // Download/Export skin
    let id_for_dl = account_id;
    let uname_for_dl = username_opt.clone();
    let mut status_dl = status_msg;

    let download_skin = move |_| {
        let Some(id) = id_for_dl else { return };
        let uname = uname_for_dl.clone().unwrap_or_else(|| "skin".to_string());
        let uuid_str = id.to_string();

        spawn(async move {
            let skin_bytes =
                if let Ok(skin_path) = oneclient_common::paths::skin_file_path(&uuid_str) {
                    polyio::read(&skin_path).await.ok()
                } else {
                    None
                };

            let bytes_to_save = match skin_bytes {
                Some(b) => b,
                None => crate::AppAssets::get_bytes("steve.png")
                    .map(|b| b.to_vec())
                    .unwrap_or_default(),
            };

            if let Some(handle) = rfd::AsyncFileDialog::new()
                .set_title("Save Skin PNG")
                .set_file_name(format!("{uname}_skin.png"))
                .add_filter("PNG Image (*.png)", &["png"])
                .save_file()
                .await
            {
                if polyio::write(handle.path(), &bytes_to_save).await.is_ok() {
                    status_dl.set(Some(("Skin exported successfully!".to_string(), false)));
                } else {
                    status_dl.set(Some(("Failed to save skin file".to_string(), true)));
                }
            }
        });
    };

    // Remove / Reset skin
    let id_for_reset = account_id;
    let uname_for_reset = username_opt.clone();
    let mut status_reset = status_msg;

    let reset_skin = move |_| {
        let Some(id) = id_for_reset else { return };
        let uname = uname_for_reset.clone();
        let uuid_str = id.to_string();

        spawn(async move {
            let _ = delete_account_skin(&uuid_str, uname.as_deref()).await;
            status_reset.set(Some(("Skin reset to default Steve/Alex".to_string(), false)));
        });
    };

    let mut open_url_modal = show_url_modal;
    let open_from_url = move |_| {
        open_url_modal.set(true);
    };

    rect()
        .vertical()
        .width(Size::flex(1.0))
        .height(Size::fill())
        .overflow(Overflow::Clip)
        .spacing(18.)
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .child(
                    label()
                        .text("Skin Manager")
                        .font_size(32.)
                        .font_weight(FontWeight::BOLD)
                        .color(colors::fg_primary()),
                )
                .child(
                    label()
                        .text(format!("Managing skin for: {username}"))
                        .font_size(14.)
                        .color(colors::fg_secondary()),
                ),
        )
        .maybe_child((*status_msg.read()).as_ref().map(|(msg, is_err)| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(8.)
                .padding(Gaps::new_symmetric(8., 12.))
                .corner_radius(CornerRadius::new_all(8.))
                .background(if *is_err {
                    colors::danger().with_a(30)
                } else {
                    colors::success().with_a(30)
                })
                .border(border_all_color(
                    1.,
                    if *is_err {
                        colors::danger()
                    } else {
                        colors::success()
                    },
                ))
                .child(
                    Icon::new(if *is_err {
                        IconType::AlertTriangle
                    } else {
                        IconType::CheckCircle
                    })
                    .size(16.)
                    .color(if *is_err {
                        colors::danger()
                    } else {
                        colors::success()
                    }),
                )
                .child(
                    label()
                        .text(msg.clone())
                        .font_size(13.)
                        .color(colors::fg_primary()),
                )
                .into_element()
        }))
        .child(
            rect()
                .horizontal()
                .content(Content::wrap_spacing(8.))
                .spacing(8.)
                .child(
                    Button::new()
                        .primary()
                        .on_press(import_from_file)
                        .child(Icon::new(IconType::FilePlus02).size(16.))
                        .text("Upload File (.png)"),
                )
                .child(
                    Button::new()
                        .secondary()
                        .on_press(open_from_url)
                        .child(Icon::new(IconType::Globe01).size(16.))
                        .text("From Username / URL"),
                )
                .child(
                    Button::new()
                        .secondary()
                        .on_press(download_skin)
                        .child(Icon::new(IconType::Download01).size(16.))
                        .text("Export Skin"),
                )
                .child(
                    Button::new()
                        .ghost()
                        .on_press(reset_skin)
                        .child(Icon::new(IconType::Trash01).size(16.))
                        .text("Reset Skin"),
                ),
        )
        .child(
            rect()
                .vertical()
                .spacing(8.)
                .child(section_label("SWITCH ACCOUNT"))
                .child(
                    rect()
                        .horizontal()
                        .content(Content::wrap_spacing(8.))
                        .spacing(8.)
                        .children(accounts.into_iter().map(|acc| {
                            let id = acc.id;
                            let is_sel = selected_id.read().map_or(false, |sel| sel == id);
                            let mut sel_state = selected_id;
                            rect()
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(6.)
                                .padding(Gaps::new_symmetric(6., 12.))
                                .corner_radius(CornerRadius::new_all(8.))
                                .background(if is_sel {
                                    colors::brand().with_a(50)
                                } else {
                                    colors::page_elevated()
                                })
                                .border(border_all_color(
                                    1.,
                                    if is_sel {
                                        colors::brand()
                                    } else {
                                        colors::component_border()
                                    },
                                ))
                                .cursor(CursorIcon::Pointer)
                                .on_press(move |_| sel_state.set(Some(id)))
                                .child(
                                    label()
                                        .text(acc.username)
                                        .font_size(13.)
                                        .color(colors::fg_primary()),
                                )
                                .into_element()
                        })),
                ),
        )
        .child(
            rect()
                .vertical()
                .spacing(10.)
                .padding(Gaps::new_all(14.))
                .corner_radius(CornerRadius::new_all(10.))
                .background(colors::page_elevated())
                .border(border_all_color(1., colors::component_border()))
                .child(
                    label()
                        .text("💡 Offline Skin Tips")
                        .font_size(14.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(colors::fg_primary()),
                )
                .child(
                    label()
                        .text("• Supports all standard Minecraft skins (64x64 or legacy 64x32 PNGs).")
                        .font_size(12.)
                        .color(colors::fg_secondary()),
                )
                .child(
                    label()
                        .text("• Interactive 3D preview: Click & drag on the character to rotate!")
                        .font_size(12.)
                        .color(colors::fg_secondary()),
                )
                .child(
                    label()
                        .text("• Skins automatically sync with CustomSkinLoader for in-game rendering on offline servers.")
                        .font_size(12.)
                        .color(colors::fg_secondary()),
                ),
        )
        .into_element()
}

fn url_modal(
    selected_id: State<Option<uuid::Uuid>>,
    status_msg: State<Option<(String, bool)>>,
    show_url_modal: State<bool>,
    is_loading: State<bool>,
) -> impl IntoElement {
    let input_text = use_state(String::new);
    let mut modal_open = show_url_modal;
    let mut status = status_msg;
    let mut loading = is_loading;

    let target_uuid = (*selected_id.peek()).map(|id| id.to_string());

    let apply_url = move |_| {
        let Some(ref uuid_str) = target_uuid else {
            return;
        };
        let uuid_str = uuid_str.clone();
        let query = input_text.peek().trim().to_string();
        if query.is_empty() {
            return;
        }

        loading.set(true);
        modal_open.set(false);

        spawn(async move {
            match fetch_skin_online(&query).await {
                Ok((bytes, is_slim)) => {
                    let _ = save_account_skin(&uuid_str, None, &bytes, is_slim).await;
                    status.set(Some((
                        format!("Skin for \"{query}\" applied successfully!"),
                        false,
                    )));
                }
                Err(err) => {
                    status.set(Some((err, true)));
                }
            }
            loading.set(false);
        });
    };

    OverlayPopup::new()
        .position(Position::new_global().top(140.).left(280.))
        .on_close(move |()| modal_open.set(false))
        .child(
            rect()
                .vertical()
                .width(Size::px(380.))
                .padding(Gaps::new_all(20.))
                .spacing(16.)
                .corner_radius(CornerRadius::new_all(12.))
                .background(colors::page_elevated())
                .border(border_all_color(1., colors::component_border()))
                .shadow(Shadow::from((0., 8., 32., 0., Color::from_argb(140, 0, 0, 0))))
                .child(
                    label()
                        .text("Import Skin by Username or URL")
                        .font_size(16.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(colors::fg_primary()),
                )
                .child(
                    label()
                        .text("Enter any Minecraft player name (e.g. Technoblade, Notch) or a direct skin image URL:")
                        .font_size(12.)
                        .color(colors::fg_secondary()),
                )
                .child(
                    TextInput::new(input_text)
                        .placeholder("Username (e.g. Dream) or https://...")
                        .width(Size::fill()),
                )
                .child(
                    rect()
                        .horizontal()
                        .main_align(Alignment::End)
                        .spacing(8.)
                        .child(
                            Button::new()
                                .ghost()
                                .on_press(move |_| modal_open.set(false))
                                .text("Cancel"),
                        )
                        .child(
                            Button::new()
                                .primary()
                                .on_press(apply_url)
                                .text("Apply Skin"),
                        ),
                ),
        )
        .into_element()
}

fn section_label(text: &'static str) -> impl IntoElement {
    label()
        .text(text)
        .font_size(11.)
        .font_weight(FontWeight::SEMI_BOLD)
        .color(colors::fg_secondary())
        .into_element()
}
