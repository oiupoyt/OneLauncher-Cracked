use freya::prelude::*;
use freya::router::RouterContext;
use oneclient_auth::MinecraftAccount;

use crate::components::{
    Avatar, Button, Icon, IconType, TextInput, use_microsoft_login,
};
use crate::hooks::{
    AddOfflineAccountKeys, try_default_account, use_add_offline_account, use_current_account,
};
use crate::routes::Route;
use crate::theme::colors;
use crate::view::onboarding::{
    onboarding_illustration, onboarding_page, step_heading,
};

#[derive(PartialEq)]
pub struct OnboardingAccount;

impl Component for OnboardingAccount {
    fn render(&self) -> impl IntoElement {
        let account_query = use_current_account();
        let msa = use_microsoft_login();
        let add_offline = use_add_offline_account();

        let offline_name = use_state(|| "Player".to_string());
        let is_editing = use_state(|| false);

        let account = try_default_account(&account_query);
        let show_entry = account.is_none() || *is_editing.read();

        let content = rect()
            .vertical()
            .width(Size::fill())
            .spacing(24.)
            .child(step_heading(
                "Account",
                "Enter your offline username to play, or sign in with your Microsoft account.",
            ))
            .child(if show_entry {
                let start = msa.clone();
                let add_offline_click = add_offline.clone();
                let mut is_editing_save = is_editing.clone();
                let offline_name_read = offline_name.clone();

                rect()
                    .vertical()
                    .spacing(16.)
                    .child(
                        rect()
                            .vertical()
                            .spacing(8.)
                            .child(
                                label()
                                    .text("Choose Username:")
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(colors::fg_primary()),
                            )
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(8.)
                                    .cross_align(Alignment::Center)
                                    .child(
                                        TextInput::new(offline_name.clone())
                                            .placeholder("Enter username (e.g. Steve)")
                                            .width(Size::px(240.)),
                                    )
                                    .child(
                                        Button::new()
                                            .primary()
                                            .large()
                                            .on_press(move |_| {
                                                let target = sanitize_offline_username(&offline_name_read.read());
                                                add_offline_click.mutate(AddOfflineAccountKeys {
                                                    username: target,
                                                });
                                                is_editing_save.set(false);
                                            })
                                            .text("Set & Play"),
                                    ),
                            )
                            .child(
                                label()
                                    .text("3–16 characters (letters, numbers, underscore)")
                                    .font_size(11.)
                                    .color(colors::fg_secondary()),
                            ),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .child(
                                label()
                                    .text("— or sign in with Microsoft —")
                                    .font_size(12.)
                                    .color(colors::fg_secondary()),
                            ),
                    )
                    .child(sign_in_card(msa.pending, msa.error.clone(), move |_| {
                        start.start()
                    }))
                    .into_element()
            } else {
                let mut is_editing_toggle = is_editing.clone();
                let account = account.as_ref().unwrap();

                rect()
                    .vertical()
                    .spacing(16.)
                    .child(account_preview(account))
                    .child(
                        rect()
                            .horizontal()
                            .spacing(8.)
                            .child(
                                Button::new()
                                    .secondary()
                                    .on_press(move |_| {
                                        is_editing_toggle.set(true);
                                    })
                                    .text("Change Username"),
                            ),
                    )
                    .into_element()
            })
            .into_element();

        let add_offline_nav = add_offline.clone();
        let offline_name_nav = offline_name.clone();
        let is_editing_nav = is_editing.clone();
        let account_clone = account.clone();

        let on_next = move |_| {
            if account_clone.is_none() || *is_editing_nav.read() {
                let target = sanitize_offline_username(&offline_name_nav.read());
                add_offline_nav.mutate(AddOfflineAccountKeys {
                    username: target,
                });
            }
            let _ = RouterContext::get().replace(Route::OnboardingBundles {});
        };

        let nav = rect()
            .horizontal()
            .width(Size::fill())
            .main_align(Alignment::End)
            .cross_align(Alignment::Center)
            .spacing(12.)
            .padding(Gaps::new(0., 40., 32., 40.))
            .child(
                Button::new()
                    .secondary()
                    .width(Size::px(128.))
                    .on_press(move |_| {
                        let _ = RouterContext::get().replace(Route::OnboardingLanguage {});
                    })
                    .text("Back"),
            )
            .child(
                Button::new()
                    .primary()
                    .width(Size::px(140.))
                    .enabled(true)
                    .on_press(on_next)
                    .text("Next")
                    .child(Icon::new(IconType::ArrowRight).size(16.)),
            );

        let page = onboarding_page(
            onboarding_illustration(IconType::OnboardingAccount),
            content,
            nav,
        );

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(page)
            .maybe_child(msa.popup())
    }
}

fn account_preview(account: &MinecraftAccount) -> impl IntoElement {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(24.)
        .child(
            rect()
                .horizontal()
                .spacing(12.)
                .cross_align(Alignment::Center)
                .child(
                    Avatar::new(account.id.to_string())
                        .width(Size::px(48.))
                        .height(Size::px(48.)),
                )
                .child(
                    rect()
                        .vertical()
                        .spacing(4.)
                        .child(
                            label()
                                .text(account.username.clone())
                                .font_size(16.)
                                .font_weight(FontWeight::SEMI_BOLD)
                                .color(colors::fg_primary()),
                        )
                        .child(
                            label()
                                .text(account.id.to_string())
                                .font_size(12.)
                                .color(colors::fg_secondary()),
                        ),
                ),
        )
        .into_element()
}

fn sign_in_card(
    pending: bool,
    error: Option<String>,
    on_add: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    rect()
        .vertical()
        .spacing(12.)
        .cross_align(Alignment::Start)
        .child(
            Button::new()
                .primary()
                .large()
                .enabled(!pending)
                .on_press(on_add)
                .child(Icon::new(IconType::Globe01).size(16.))
                .text(if pending {
                    "Signing in..."
                } else {
                    "Add Account"
                }),
        )
        .maybe_child(error.map(|message| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(6.)
                .child(
                    Icon::new(IconType::AlertTriangle)
                        .size(13.)
                        .color(colors::danger()),
                )
                .child(label().text(message).font_size(12.).color(colors::danger()))
                .into_element()
        }))
        .into_element()
}

fn sanitize_offline_username(input: &str) -> String {
    let trimmed = input.trim();
    let sanitized: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if sanitized.len() < 3 {
        "Player".to_string()
    } else if sanitized.len() > 16 {
        sanitized[..16].to_string()
    } else {
        sanitized
    }
}

