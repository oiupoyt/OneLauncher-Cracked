use freya::prelude::*;
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
    onboarding_illustration, onboarding_nav, onboarding_page, step_heading,
};

#[derive(PartialEq)]
pub struct OnboardingAccount;

impl Component for OnboardingAccount {
    fn render(&self) -> impl IntoElement {
        let account_query = use_current_account();
        let msa = use_microsoft_login();
        let add_offline = use_add_offline_account();

        let offline_name = use_state(|| "Player".to_string());

        let account = try_default_account(&account_query);
        let has_account = account.is_some();

        let content = rect()
            .vertical()
            .width(Size::fill())
            .spacing(24.)
            .child(step_heading(
                "Account",
                "Choose an offline username to play, or sign in with your Microsoft account.",
            ))
            .child(match &account {
                Some(account) => account_preview(account).into_element(),
                None => {
                    let start = msa.clone();
                    let add_offline_click = add_offline.clone();
                    let name_val = offline_name.read().trim().to_string();
                    let target_name = if name_val.is_empty() {
                        "Player".to_string()
                    } else {
                        name_val
                    };

                    rect()
                        .vertical()
                        .spacing(16.)
                        .child(
                            rect()
                                .vertical()
                                .spacing(8.)
                                .child(
                                    label()
                                        .text("Play Offline:")
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
                                                .placeholder("Enter username")
                                                .width(Size::px(220.)),
                                        )
                                        .child(
                                            Button::new()
                                                .primary()
                                                .large()
                                                .on_press(move |_| {
                                                    add_offline_click.mutate(AddOfflineAccountKeys {
                                                        username: target_name.clone(),
                                                    });
                                                })
                                                .text("Play Offline"),
                                        ),
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
                }
            })
            .into_element();

        let page = onboarding_page(
            onboarding_illustration(IconType::OnboardingAccount),
            content,
            onboarding_nav(
                Some(Route::OnboardingLanguage {}),
                Route::OnboardingBundles {},
                has_account,
            ),
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
