use freya::prelude::*;
use oneclient_auth::MinecraftAccount;

use crate::components::{Button, Icon, IconType, PlayerModel};
use crate::hooks::{
    delete_account_skin, save_account_skin, try_accounts, try_default_account, use_accounts,
    use_current_account, use_custom_skin,
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

        let account_uuid = active_account.as_ref().map(|a| a.id.to_string());
        let account_id = active_account.as_ref().map(|a| a.id);
        let username = active_account.as_ref().map(|a| a.username.clone());

        let status_msg = use_state(|| None::<(String, bool)>); // (message, is_error)
        let is_loading = use_state(|| false);

        rect()
            .horizontal()
            .width(Size::fill())
            .height(Size::fill())
            .overflow(Overflow::Clip)
            .padding(40.)
            .spacing(28.)
            .child(PreviewPanel {
                account_uuid,
                account_id,
                username,
                status_msg,
            })
            .child(side_panel(
                active_account,
                accounts,
                selected_id,
                status_msg,
                is_loading,
            ))
    }
}

#[derive(PartialEq, Clone)]
struct PreviewPanel {
    account_uuid: Option<String>,
    account_id: Option<uuid::Uuid>,
    username: Option<String>,
    status_msg: State<Option<(String, bool)>>,
}

impl Component for PreviewPanel {
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.account_id)
    }

    fn render(&self) -> impl IntoElement {
        let account_uuid = self.account_uuid.clone();
        let custom_query = use_custom_skin(account_uuid.clone().unwrap_or_default());
        let custom_data = crate::hooks::settled_or_loading(&custom_query);
        let is_slim = custom_data.as_ref().is_some_and(|d| d.is_slim);
        let has_custom = custom_data.as_ref().is_some_and(|d| d.has_custom);

        let acc_id_copy = self.account_id;
        let username_copy = self.username.clone();
        let status_msg = self.status_msg;

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
                            let _ = save_account_skin(&uuid_str, uname.as_deref(), &bytes, false)
                                .await;
                            status.set(Some((
                                "Model set to Classic (4px arms)".to_string(),
                                false,
                            )));
                        } else if let Some(steve_bytes) = crate::AppAssets::get_bytes("steve.png") {
                            let _ =
                                save_account_skin(&uuid_str, uname.as_deref(), &steve_bytes, false)
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
                            let _ =
                                save_account_skin(&uuid_str, uname.as_deref(), &bytes, true).await;
                            status.set(Some(("Model set to Slim (3px arms)".to_string(), false)));
                        } else if let Some(alex_bytes) = crate::AppAssets::get_bytes("alex.png") {
                            let _ =
                                save_account_skin(&uuid_str, uname.as_deref(), &alex_bytes, true)
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
}

fn side_panel(
    active_account: Option<MinecraftAccount>,
    accounts: Vec<MinecraftAccount>,
    selected_id: State<Option<uuid::Uuid>>,
    status_msg: State<Option<(String, bool)>>,
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

    // Reset skin to default
    let id_for_reset = account_id;
    let uname_for_reset = username_opt.clone();
    let mut status_reset = status_msg;

    let reset_skin = move |_| {
        let Some(id) = id_for_reset else { return };
        let uname = uname_for_reset.clone();
        let uuid_str = id.to_string();

        spawn(async move {
            match delete_account_skin(&uuid_str, uname.as_deref()).await {
                Ok(()) => {
                    status_reset
                        .set(Some(("Skin reset to default skin.".to_string(), false)));
                }
                Err(err) => {
                    status_reset.set(Some((err, true)));
                }
            }
        });
    };

    rect()
        .vertical()
        .width(Size::flex(1.0))
        .height(Size::fill())
        .spacing(24.)
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .child(
                    label()
                        .text("Skin Customizer")
                        .font_size(24.)
                        .font_weight(FontWeight::BOLD)
                        .color(colors::fg_primary()),
                )
                .child(
                    label()
                        .text(format!("Managing skin for {username}"))
                        .font_size(13.)
                        .color(colors::fg_secondary()),
                ),
        )
        // Status message notification banner
        .maybe_child(status_msg.read().as_ref().map(|(msg, is_err)| {
            let is_err = *is_err;
            rect()
                .horizontal()
                .width(Size::fill())
                .padding(Gaps::new_symmetric(10., 14.))
                .corner_radius(CornerRadius::new_all(8.))
                .background(if is_err {
                    colors::danger().with_a(30)
                } else {
                    colors::success().with_a(30)
                })
                .border(border_all_color(
                    1.,
                    if is_err {
                        colors::danger()
                    } else {
                        colors::success()
                    },
                ))
                .child(
                    label()
                        .text(msg.clone())
                        .font_size(13.)
                        .color(if is_err {
                            colors::danger()
                        } else {
                            colors::success()
                        }),
                )
                .into_element()
        }))
        // Account selector dropdown/buttons
        .child(account_selector(accounts, selected_id))
        // Action buttons
        .child(
            rect()
                .vertical()
                .spacing(12.)
                .child(
                    Button::new()
                        .primary()
                        .enabled(!*is_loading.read() && account_id.is_some())
                        .on_press(import_from_file)
                        .child(
                            rect()
                                .horizontal()
                                .spacing(8.)
                                .cross_align(Alignment::Center)
                                .child(Icon::new(IconType::FilePlus02).size(18.))
                                .child(label().text("Upload PNG from Computer").font_size(14.)),
                        ),
                )
                .child(
                    rect()
                        .horizontal()
                        .spacing(12.)
                        .child(
                            Button::new()
                                .ghost()
                                .enabled(!*is_loading.read() && account_id.is_some())
                                .on_press(download_skin)
                                .child(
                                    rect()
                                        .horizontal()
                                        .spacing(8.)
                                        .cross_align(Alignment::Center)
                                        .child(Icon::new(IconType::Download01).size(16.))
                                        .child(label().text("Save PNG").font_size(13.)),
                                ),
                        )
                        .child(
                            Button::new()
                                .ghost()
                                .enabled(!*is_loading.read() && account_id.is_some())
                                .on_press(reset_skin)
                                .child(
                                    rect()
                                        .horizontal()
                                        .spacing(8.)
                                        .cross_align(Alignment::Center)
                                        .child(
                                            Icon::new(IconType::Trash01)
                                                 .size(16.)
                                                 .color(colors::danger()),
                                        )
                                        .child(
                                            label()
                                                .text("Reset to Default")
                                                .font_size(13.)
                                                .color(colors::danger()),
                                        ),
                                ),
                        ),
                ),
        )
        // Information note
        .child(
            rect()
                .vertical()
                .padding(Gaps::new_all(14.))
                .corner_radius(CornerRadius::new_all(8.))
                .background(colors::component_bg())
                .spacing(6.)
                .child(
                    label()
                        .text("Offline Skin System")
                        .font_size(13.)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .color(colors::fg_primary()),
                )
                .child(
                    label()
                        .text("Custom skins are saved locally to your profile and seamlessly loaded into singleplayer and offline servers.")
                        .font_size(12.)
                        .color(colors::fg_secondary()),
                ),
        )
        .into_element()
}

fn account_selector(
    accounts: Vec<MinecraftAccount>,
    selected_id: State<Option<uuid::Uuid>>,
) -> impl IntoElement {
    let mut selected_state = selected_id;

    rect()
        .vertical()
        .spacing(8.)
        .child(
            label()
                .text("SELECT ACCOUNT")
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(colors::fg_secondary()),
        )
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .content(Content::Flex)
                .children(accounts.into_iter().map(|acc| {
                    let id = acc.id;
                    let uname = acc.username.clone();
                    let is_active = *selected_state.read() == Some(id);

                    rect()
                        .center()
                        .padding(Gaps::new_symmetric(8., 16.))
                        .corner_radius(CornerRadius::new_all(8.))
                        .background(if is_active {
                            colors::brand()
                        } else {
                            colors::component_bg()
                        })
                        .border(border_all_color(
                            1.,
                            if is_active {
                                colors::brand()
                            } else {
                                colors::component_border()
                            },
                        ))
                        .cursor(CursorIcon::Pointer)
                        .on_press(move |_| selected_state.set(Some(id)))
                        .child(
                            label()
                                .text(uname)
                                .font_size(13.)
                                .font_weight(if is_active {
                                    FontWeight::SEMI_BOLD
                                } else {
                                    FontWeight::NORMAL
                                })
                                .color(colors::fg_primary()),
                        )
                        .into_element()
                })),
        )
}
