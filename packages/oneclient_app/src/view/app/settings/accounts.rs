use chrono::{DateTime, Utc};
use freya::animation::{AnimNum, Function, OnFinish, use_animation};
use freya::prelude::*;
use freya::query::{MutationCapability, MutationStateData, UseMutation};
use oneclient_auth::{AccountKind, MinecraftAccount};
use uuid::Uuid;

use super::{section_header, settings_page};
use crate::components::{
    Avatar, Button, Icon, IconType, OverlayPopup, PlayerModel, TextInput, use_microsoft_login,
};
use crate::hooks::{
    AddOfflineAccountKeys, RefreshAccountKeys, RemoveAccountKeys, SetDefaultAccountKeys,
    accounts_have_microsoft, try_accounts, try_default_account, use_accounts,
    use_add_offline_account, use_current_account, use_refresh_account, use_remove_account,
    use_set_default_account,
};
use crate::theme::colors;
use crate::ui::border_all_color;

/// The shader scales the model off `res.y` alone so height makes the player bigger width only needs to clear the arms
const MODEL_WIDTH_PX: f32 = 160.;
const MODEL_HEIGHT_PX: f32 = 264.;

const AVATAR_SIZE_PX: f32 = 36.;

#[derive(PartialEq)]
pub struct SettingsAccounts;

impl Component for SettingsAccounts {
    fn render(&self) -> impl IntoElement {
        let accounts_query = use_accounts();
        let default_query = use_current_account();

        let msa = use_microsoft_login();
        let add_offline = use_add_offline_account();
        let set_default = use_set_default_account();
        let remove = use_remove_account();
        let refresh = use_refresh_account();

        let username = use_state(String::new);
        let mut show_offline = use_state(|| false);
        let edit_account = use_state(|| None::<(Uuid, String)>);
        let edit_name = use_state(|| String::new());

        let accounts = try_accounts(&accounts_query).unwrap_or_default();
        let default_account = try_default_account(&default_query);
        let default_id = default_account.as_ref().map(|a| a.id);
        let has_microsoft = accounts_have_microsoft(&accounts);

        let offline_name = username.read().trim().to_string();
        let offline_uuid = (!offline_name.is_empty())
            .then(|| oneclient_auth::offline_uuid(&offline_name).to_string());

        let offline_error = mutation_err_text(&add_offline);

        let on_confirm_offline = move |_| {
            let name = username.peek().trim().to_string();
            if name.is_empty() {
                return;
            }
            let mut show_offline = show_offline;
            let mut username = username;
            spawn(async move {
                let reader = add_offline
                    .mutate_async(AddOfflineAccountKeys { username: name })
                    .await;
                if matches!(
                    &*reader.state(),
                    MutationStateData::Settled { res: Ok(_), .. }
                ) {
                    show_offline.set(false);
                    username.set(String::new());
                }
            });
        };

        let mut rows: Vec<Element> = accounts
            .iter()
            .map(|account| {
                AccountRow {
                    id: account.id,
                    username: account.username.clone(),
                    kind: account.kind,
                    expires: account.expires,
                    is_default: Some(account.id) == default_id,
                    set_default,
                    remove,
                    refresh,
                    edit_account,
                    edit_name,
                }
                .into_element()
            })
            .collect();
        if rows.is_empty() {
            rows.push(empty_state());
        }

        settings_page()
            .child(hero(
                default_account,
                has_microsoft,
                msa.pending,
                msa.error.clone(),
                move |_| show_offline.set(true),
                {
                    let msa = msa.clone();
                    move |_| msa.start()
                },
            ))
            .child(section_header("YOUR ACCOUNTS"))
            .children(rows)
            .maybe_child(show_offline.read().then(|| {
                offline_dialog(
                    username,
                    offline_uuid,
                    offline_error,
                    on_confirm_offline,
                    show_offline,
                )
            }))
            .maybe_child(edit_account.read().as_ref().map(|(id, _)| {
                let id = *id;
                let mut edit_account = edit_account;
                let edit_name = edit_name;
                OverlayPopup::new()
                    .on_close(move |()| edit_account.set(None))
                    .child(
                        rect()
                            .width(Size::window_percent(100.))
                            .height(Size::window_percent(100.))
                            .center()
                            .child(
                                rect()
                                    .vertical()
                                    .width(Size::px(380.))
                                    .max_width(Size::window_percent(90.))
                                    .spacing(16.)
                                    .padding(Gaps::new_all(20.))
                                    .corner_radius(CornerRadius::new_all(16.))
                                    .background(colors::page_elevated())
                                    .border(border_all_color(1., colors::component_border()))
                                    .child(
                                        label()
                                            .text("Edit offline username")
                                            .font_size(18.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(colors::fg_primary()),
                                    )
                                    .child(
                                        rect()
                                            .vertical()
                                            .width(Size::fill())
                                            .spacing(6.)
                                            .child(field_label("New Username"))
                                            .child(TextInput::new(edit_name).placeholder("Enter new username")),
                                    )
                                    .child(
                                        rect()
                                            .horizontal()
                                            .width(Size::fill())
                                            .main_align(Alignment::End)
                                            .spacing(8.)
                                            .child(
                                                Button::new()
                                                    .ghost()
                                                    .on_press(move |_| edit_account.set(None))
                                                    .text("Cancel"),
                                            )
                                            .child(
                                                Button::new()
                                                    .primary()
                                                    .on_press(move |_| {
                                                        let new_name = edit_name.peek().trim().to_string();
                                                        if !new_name.is_empty() {
                                                            let mut edit_account = edit_account;
                                                            spawn(async move {
                                                                if let Ok(state) = crate::launcher::state() {
                                                                    let _ = state.auth.remove_account(id).await;
                                                                    if let Ok(new_acc) = state.auth.add_offline_account(new_name).await {
                                                                        let _ = state.auth.set_default_account(Some(new_acc.id)).await;
                                                                        crate::hooks::invalidate_auth_queries(Some(new_acc.id)).await;
                                                                    }
                                                                }
                                                                edit_account.set(None);
                                                            });
                                                        }
                                                    })
                                                    .text("Save"),
                                            ),
                                    ),
                            ),
                    )
            }))
            .maybe_child(msa.popup())
            .into_element()
    }
}

fn hero(
    account: Option<MinecraftAccount>,
    _has_microsoft: bool,
    microsoft_pending: bool,
    error: Option<String>,
    on_open_offline: impl FnMut(Event<PressEventData>) + 'static,
    on_add_microsoft: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    let (name, subtitle) = match &account {
        Some(account) => (account.username.clone(), kind_label(account.kind)),
        None => (
            "No active account".to_string(),
            "Add an account to start playing",
        ),
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(16.)
        .padding(Gaps::new_all(16.))
        .corner_radius(CornerRadius::new_all(12.))
        .background(colors::page_elevated())
        .child(model_frame(account.as_ref().map(|account| account.id)))
        .child(
            rect()
                .vertical()
                .width(Size::flex(1.0))
                .spacing(16.)
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(2.)
                        .child(
                            label()
                                .text("ACTIVE ACCOUNT")
                                .font_size(11.)
                                .font_weight(FontWeight::MEDIUM)
                                .color(colors::fg_secondary()),
                        )
                        .child(
                            label()
                                .text(name)
                                .font_size(24.)
                                .font_weight(FontWeight::SEMI_BOLD)
                                .max_lines(1)
                                .color(colors::fg_primary()),
                        )
                        .child(
                            label()
                                .text(subtitle)
                                .font_size(12.)
                                .color(colors::fg_secondary()),
                        ),
                )
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(8.)
                        .child(
                            // The two buttons overflow the ~245px beside the model at the 800px minimum so they wrap
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .content(Content::wrap_spacing(8.))
                                .spacing(8.)
                                .child(
                                    Button::new()
                                        .primary()
                                        .enabled(!microsoft_pending)
                                        .on_press(on_add_microsoft)
                                        .child(Icon::new(IconType::Globe01).size(16.))
                                        .text(if microsoft_pending {
                                            "Signing in..."
                                        } else {
                                            "Add Microsoft"
                                        }),
                                )
                                .child(
                                    Button::new()
                                        .secondary()
                                        .enabled(true)
                                        .on_press(on_open_offline)
                                        .child(Icon::new(IconType::Plus).size(16.))
                                        .text("Add offline"),
                                )
                                .child(
                                    Button::new()
                                        .secondary()
                                        .on_press(|_| {
                                            let _ = freya::router::RouterContext::get().push(crate::Route::AccountSkins {});
                                        })
                                        .child(Icon::new(IconType::Colors).size(16.))
                                        .text("Change Skin"),
                                ),
                        )
                        .map(error, |el, msg| {
                            el.child(hint_line(IconType::AlertTriangle, msg, colors::danger()))
                        })
                        .maybe(false, |el| {
                            el.child(hint_line(
                                IconType::InfoCircle,
                                "Add a Microsoft account before creating offline accounts."
                                    .to_string(),
                                colors::fg_secondary(),
                            ))
                        }),
                ),
        )
        .into_element()
}

fn model_frame(id: Option<Uuid>) -> impl IntoElement {
    rect()
        .vertical()
        .cross_align(Alignment::Center)
        .spacing(6.)
        .child(
            rect()
                .width(Size::px(MODEL_WIDTH_PX))
                .height(Size::px(MODEL_HEIGHT_PX))
                .center()
                .overflow(Overflow::Clip)
                .corner_radius(CornerRadius::new_all(12.))
                .background(colors::component_bg())
                .border(border_all_color(1., colors::component_border()))
                .child(match id {
                    Some(id) => PlayerModel::new(id)
                        .yaw(-0.5)
                        .width(Size::fill())
                        .height(Size::fill())
                        .into_element(),
                    None => Icon::new(IconType::Users01)
                        .size(28.)
                        .color(colors::fg_secondary())
                        .into_element(),
                }),
        )
        .maybe_child(id.map(|_| {
            label()
                .text("Drag to rotate")
                .font_size(10.)
                .color(colors::fg_secondary())
                .into_element()
        }))
        .into_element()
}

fn mutation_err_text<M>(mutation: &UseMutation<M>) -> Option<String>
where
    M: MutationCapability,
    M::Err: std::fmt::Display,
{
    match &*mutation.read().state() {
        MutationStateData::Settled { res: Err(err), .. } => Some(err.to_string()),
        MutationStateData::Loading {
            res: Some(Err(err)),
        } => Some(err.to_string()),
        _ => None,
    }
}

fn offline_dialog(
    username: State<String>,
    uuid_preview: Option<String>,
    error: Option<String>,
    on_confirm: impl FnMut(Event<PressEventData>) + 'static,
    mut show_offline: State<bool>,
) -> impl IntoElement {
    OverlayPopup::new()
        .on_close(move |()| show_offline.set(false))
        .child(
            rect()
                .width(Size::window_percent(100.))
                .height(Size::window_percent(100.))
                .center()
                .child(
                    rect()
                        .vertical()
                        .width(Size::px(380.))
                        .max_width(Size::window_percent(90.))
                        .spacing(16.)
                        .padding(Gaps::new_all(20.))
                        .corner_radius(CornerRadius::new_all(16.))
                        .background(colors::page_elevated())
                        .border(border_all_color(1., colors::component_border()))
                        .child(
                            label()
                                .text("Add offline account")
                                .font_size(18.)
                                .font_weight(FontWeight::SEMI_BOLD)
                                .color(colors::fg_primary()),
                        )
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(6.)
                                .child(field_label("Username"))
                                .child(TextInput::new(username).placeholder("Offline username")),
                        )
                        .child(
                            rect()
                                .vertical()
                                .width(Size::fill())
                                .spacing(6.)
                                .child(field_label("UUID"))
                                .child(
                                    label()
                                        .text(uuid_preview.unwrap_or_else(|| "—".to_string()))
                                        .font_size(12.)
                                        .color(colors::fg_secondary()),
                                ),
                        )
                        .map(error, |el, msg| {
                            el.child(hint_line(IconType::AlertTriangle, msg, colors::danger()))
                        })
                        .child(
                            rect()
                                .horizontal()
                                .width(Size::fill())
                                .main_align(Alignment::End)
                                .spacing(8.)
                                .child(
                                    Button::new()
                                        .ghost()
                                        .on_press(move |_| show_offline.set(false))
                                        .text("Cancel"),
                                )
                                .child(
                                    Button::new()
                                        .primary()
                                        .on_press(on_confirm)
                                        .child(Icon::new(IconType::Plus).size(16.))
                                        .text("Add account"),
                                ),
                        ),
                ),
        )
        .into_element()
}

fn field_label(text: &str) -> impl IntoElement {
    label()
        .text(text.to_string())
        .font_size(11.)
        .font_weight(FontWeight::MEDIUM)
        .color(colors::fg_secondary())
        .into_element()
}

/// Sits in whatever is left of the hero beside the model so the text must wrap
fn hint_line(icon: IconType, text: String, color: Color) -> impl IntoElement {
    rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(6.)
        .child(Icon::new(icon).size(13.).color(color))
        .child(
            label()
                .text(text)
                .font_size(12.)
                .width(Size::flex(1.0))
                .color(color),
        )
        .into_element()
}

const REFRESH_SPIN_TIME: u64 = 800;

const REFRESHING_OPACITY: f32 = 0.7;

struct AccountRow {
    id: Uuid,
    username: String,
    kind: AccountKind,
    expires: DateTime<Utc>,
    is_default: bool,
    set_default: crate::hooks::UseSetDefaultAccount,
    remove: crate::hooks::UseRemoveAccount,
    refresh: crate::hooks::UseRefreshAccount,
    edit_account: State<Option<(Uuid, String)>>,
    edit_name: State<String>,
}

impl PartialEq for AccountRow {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.username == other.username
            && self.kind == other.kind
            && self.expires == other.expires
            && self.is_default == other.is_default
            && self.edit_account == other.edit_account
            && self.edit_name == other.edit_name
    }
}

impl Component for AccountRow {
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.id)
    }

    fn render(&self) -> impl IntoElement {
        let id = self.id;
        let is_default = self.is_default;
        let set_default = self.set_default;
        let remove = self.remove;
        let refresh = self.refresh;

        let is_microsoft = self.kind == AccountKind::Microsoft;
        let expired = is_microsoft && self.expires <= Utc::now();

        let mut refreshing = use_state(|| false);
        let is_refreshing = *refreshing.read();

        let spin = use_animation(|conf| {
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0., 360.)
                .time(REFRESH_SPIN_TIME)
                .function(Function::Linear)
        });

        use_side_effect_with_deps(&is_refreshing, move |&is_refreshing| {
            let mut spin = spin;
            if is_refreshing {
                spin.start();
            } else {
                spin.reset();
            }
        });

        let rotation = if is_refreshing {
            spin.get().value()
        } else {
            0.
        };

        let border_color = if expired {
            colors::danger()
        } else if is_default {
            colors::brand()
        } else {
            colors::component_border()
        };

        rect()
            .horizontal()
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .content(Content::Flex)
            .spacing(12.)
            .padding(Gaps::new_all(12.))
            .corner_radius(CornerRadius::new_all(12.))
            .background(colors::page_elevated())
            .border(border_all_color(1., border_color))
            .opacity(if is_refreshing {
                REFRESHING_OPACITY
            } else {
                1.
            })
            .a11y_role(AccessibilityRole::Button)
            .maybe(!is_default, |el| {
                el.on_press(move |_| set_default.mutate(SetDefaultAccountKeys { id: Some(id) }))
            })
            .child(
                Avatar::new(id.to_string())
                    .width(Size::px(AVATAR_SIZE_PX))
                    .height(Size::px(AVATAR_SIZE_PX)),
            )
            .child(
                rect()
                    .vertical()
                    .width(Size::flex(1.0))
                    .spacing(4.)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(6.)
                            .child(
                                label()
                                    .text(self.username.clone())
                                    .font_size(15.)
                                    .font_weight(FontWeight::MEDIUM)
                                    .max_lines(1)
                                    .color(colors::fg_primary()),
                            )
                            .maybe_child(is_default.then(default_badge))
                            .maybe_child(expired.then(expired_badge)),
                    )
                    // The kind rides with the id a third badge and a full uuid do not both fit
                    .child(
                        label()
                            .text(format!("{} · {id}", kind_label(self.kind)))
                            .font_size(11.)
                            .max_lines(1)
                            .color(colors::fg_secondary()),
                    ),
            )
            .maybe_child(is_microsoft.then(|| {
                Button::new()
                    .ghost()
                    .icon()
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        if *refreshing.peek() {
                            return;
                        }
                        refreshing.set(true);
                        spawn(async move {
                            refresh.mutate_async(RefreshAccountKeys { id }).await;
                            refreshing.set(false);
                        });
                    })
                    .child(
                        rect().rotate(rotation).child(
                            Icon::new(IconType::RefreshCw01)
                                .size(18.)
                                .color(if expired {
                                    colors::danger()
                                } else {
                                    colors::fg_secondary()
                                }),
                        ),
                    )
                    .into_element()
            }))
            .child(
                Button::new()
                    .ghost()
                    .icon()
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        let _ = freya::router::RouterContext::get().push(crate::Route::AccountSkins {});
                    })
                    .child(
                        Icon::new(IconType::Colors)
                            .size(18.)
                            .color(colors::fg_primary()),
                    ),
            )
            .child({
                let is_offline = !is_microsoft;
                let mut edit_acc_state = self.edit_account;
                let mut edit_name_state = self.edit_name;
                let username_val = self.username.clone();
                let id_val = self.id;
                Button::new()
                    .ghost()
                    .icon()
                    .enabled(is_offline)
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        edit_name_state.set(username_val.clone());
                        edit_acc_state.set(Some((id_val, username_val.clone())));
                    })
                    .child(
                        Icon::new(IconType::Pencil01)
                            .size(18.)
                            .color(if is_offline {
                                colors::fg_primary()
                            } else {
                                colors::fg_secondary()
                            }),
                    )
            })
            .child(
                Button::new()
                    .ghost()
                    .icon()
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        remove.mutate(RemoveAccountKeys { id });
                    })
                    .child(
                        Icon::new(IconType::Trash01)
                            .size(18.)
                            .color(colors::fg_secondary()),
                    ),
            )
            .into_element()
    }
}

fn kind_label(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Microsoft => "Microsoft",
        AccountKind::Offline => "Offline",
    }
}

fn default_badge() -> impl IntoElement {
    badge(
        Icon::new(IconType::CheckCircle)
            .size(12.)
            .color(colors::brand())
            .into_element(),
        "Default".to_string(),
        colors::brand(),
        colors::brand(),
    )
}

fn expired_badge() -> impl IntoElement {
    badge(
        Icon::new(IconType::AlertTriangle)
            .size(12.)
            .color(colors::danger())
            .into_element(),
        "Expired".to_string(),
        colors::danger(),
        colors::danger(),
    )
}

fn badge(icon: impl IntoElement, text: String, border: Color, fg: Color) -> impl IntoElement {
    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(4.)
        .padding(Gaps::new_symmetric(2., 8.))
        .corner_radius(CornerRadius::new_all(999.))
        .border(border_all_color(1., border))
        .background(colors::component_bg())
        .child(icon)
        .child(
            label()
                .text(text)
                .font_size(10.)
                .font_weight(FontWeight::MEDIUM)
                .color(fg),
        )
        .into_element()
}

fn empty_state() -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .center()
        .padding(Gaps::new_all(32.))
        .spacing(8.)
        .corner_radius(CornerRadius::new_all(12.))
        .background(colors::page_elevated())
        .child(
            Icon::new(IconType::Users01)
                .size(32.)
                .color(colors::fg_secondary()),
        )
        .child(
            label()
                .text("No accounts yet. Add one above.")
                .font_size(14.)
                .color(colors::fg_secondary()),
        )
        .into_element()
}
