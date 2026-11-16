//! The organization's question boxes: GUI.dll's `OrganizationGUIModule_c` (made at 0x100533ba), read on 2026-11-16.
//! Six boxes, each a `DialogBox_c` with the buttons `MsgBox_Yes` and `MsgBox_No` whose answer goes back through
//! `N3InterfaceModule_t` to one `OrgClientIIR_c` message (Gamecode FUN_101266b4: the command's number, an identity,
//! the chat window's number, a text, a byte):
//!
//! | Box | Raised by | Title | Text (text.mdb 501) | Yes sends |
//! |---|---|---|---|---|
//! | `LeaveOrg` 0x10052c90 | `/org leave` (Gamecode's text command, case 0xc: `GlobalSignals_c` +0x1f4) | `Warning` (10000) | a line written into GUI.dll | 0x10, nobody |
//! | `OpenDisbandDialog` 0x10052805 | `/org disband` (case 0xe: the interface's message 0xf8) | `OrgDisband` | `Org_ConfirmDisband` | 6, the selected target |
//! | `OpenInvitationDialog` 0x100530e5 | `OrgServer` type 5 (message 0xf9) | `OrgJoin` | `Org_Invite` fed the name, two spaces, then `Org_InviteTax` fed the tax or `Org_InviteNoTax` | 0xf, the organization |
//! | `OpenLeavePayTaxQuery` 0x10052945 | `OrgServer` type 7 (message 0xfa) | `OrgLeave` | `Org_LeaveAskPayTax` fed the tax | 0x1d, the organization; **anything else sends 0x1e** |
//! | `OpenTaxChangeLeaveQuery` 0x10052af9 | `OrgServer` type 8 (message 0xfb) | `OrgLeave` | `Org_TaxChangedAskLeave` fed the tax | 0x10, the selected target |
//! | `OpenPromotionDialog` 0x10052e53 | `OrgServer` type 9 (`GlobalSignals_c` +0x24c) | `Warning` (10000) | `ConfirmPromotion` fed the member's name | 10, the member, with the byte 1 |
//!
//! The titles `OrgDisband`, `OrgLeave` and `OrgJoin` are what the code hands the window as its title, as written.
//! The leave and the promotion boxes come up in the screen's middle (`Window::MoveToCenter`), the others at the
//! pointer; here all six come up at the pointer. The tax boxes are only raised for a tax above 0 and under 1,000,000,000, and the promotion's only when
//! the member is in sight (its name is asked of the character's object).
//!
//! Not done: before the invitation's yes is sent, the original looks at one of the character's timers (number 5,
//! FUN_1006414f) and, while it runs, says the time left instead of sending; the client does not keep that timer, so
//! the server is asked every time. The leave box's line is this client's own words (the original's is in the
//! program, not in the game's texts). **None of the six was seen live**: no capture has an organization joined,
//! left, taxed or disbanded, so the layouts of the server's messages are from the code alone.

use mao_formats::ldb::{Arg, format};
use std::sync::Mutex;

/// A question to put to the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    Leave,
    Disband,
    Invitation { organization: (u32, u32), name: String, tax: u32 },
    LeavePayTax { organization: (u32, u32), tax: u32 },
    TaxChanged { tax: u32 },
    Promotion { member: (u32, u32), name: String },
}

/// Whose identity an answer's message carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Nobody,
    /// The selected target (nobody without one).
    Target,
    Identity((u32, u32)),
}

/// What an answer sends: the command's number, the identity, and the byte (1 for a confirmed promotion).
pub type Answer = (u8, At, bool);

fn text(name: &str, own: &str) -> String {
    crate::gui::xml::ldb_named(501, name, own)
}

impl Ask {
    /// The window's name for [`crate::chrome::dialog`].
    fn window(&self) -> &'static str {
        match self {
            Ask::Leave => "OrgLeave",
            Ask::Disband => "OrgDisband",
            Ask::Invitation { .. } => "OrgJoin",
            Ask::LeavePayTax { .. } => "OrgLeavePayTax",
            Ask::TaxChanged { .. } => "OrgTaxChanged",
            Ask::Promotion { .. } => "OrgPromotion",
        }
    }

    pub fn title(&self) -> String {
        match self {
            Ask::Leave | Ask::Promotion { .. } => crate::gui::xml::ldb_named(10000, "Warning", "Warning"),
            Ask::Disband => "OrgDisband".into(),
            Ask::Invitation { .. } => "OrgJoin".into(),
            Ask::LeavePayTax { .. } | Ask::TaxChanged { .. } => "OrgLeave".into(),
        }
    }

    pub fn text(&self) -> String {
        match self {
            Ask::Leave => "Do you want to leave the organization? For six hours afterwards you cannot join another.".into(),
            Ask::Disband => text("Org_ConfirmDisband", "Do you want to disband the organization?"),
            Ask::Invitation { name, tax, .. } => {
                let invite = format(&text("Org_Invite", "Do you want to join the organization %s?"), &[Arg::Text(name)]);
                let taxed = if *tax == 0 { text("Org_InviteNoTax", "It takes no tax.") } else { format(&text("Org_InviteTax", "Its tax is %u percent."), &[Arg::Int(*tax as i64)]) };
                format!("{invite}  {taxed}")
            }
            Ask::LeavePayTax { tax, .. } => format(&text("Org_LeaveAskPayTax", "You owe the organization %u credits of tax. Pay them as you leave?"), &[Arg::Int(*tax as i64)]),
            Ask::TaxChanged { tax } => format(&text("Org_TaxChangedAskLeave", "The organization's tax is now %u percent. Do you want to leave it?"), &[Arg::Int(*tax as i64)]),
            Ask::Promotion { name, .. } => format(&text("ConfirmPromotion", "Promote %s?"), &[Arg::Text(name)]),
        }
    }

    /// What the answer sends: `yes` for the first button; a box closed any other way is a no. Only the question
    /// about the tax owed sends something for a no.
    pub fn answer(&self, yes: bool) -> Option<Answer> {
        match (self, yes) {
            (Ask::Leave, true) => Some((0x10, At::Nobody, false)),
            (Ask::Disband, true) => Some((6, At::Target, false)),
            (Ask::Invitation { organization, .. }, true) => Some((0xf, At::Identity(*organization), false)),
            (Ask::LeavePayTax { organization, .. }, yes) => Some((if yes { 0x1d } else { 0x1e }, At::Identity(*organization), false)),
            (Ask::TaxChanged { .. }, true) => Some((0x10, At::Target, false)),
            (Ask::Promotion { member, .. }, true) => Some((10, At::Identity(*member), true)),
            _ => None,
        }
    }
}

/// The questions waiting, the first one shown.
static WAITING: Mutex<std::collections::VecDeque<Ask>> = Mutex::new(std::collections::VecDeque::new());

/// Puts a question to the player (any thread). The same question is not queued twice.
pub fn ask(question: Ask) {
    let mut waiting = WAITING.lock().unwrap_or_else(|e| e.into_inner());
    if !waiting.contains(&question) {
        eprintln!("organization box: {question:?}");
        waiting.push_back(question);
    }
}

/// A question the server's message asks for, with `name_of` for a character in sight: nothing for a tax outside
/// the original's range (0, or 1,000,000,000 and over) and for a promotion of somebody not in sight.
pub fn from_server(asked: mao_net::n3::OrgAsk, name_of: impl Fn(u32) -> Option<String>) -> Option<Ask> {
    let taxed = |tax: u32| tax != 0 && tax < 1_000_000_000;
    match asked {
        mao_net::n3::OrgAsk::Invitation { organization, name, tax } => (organization != (0, 0)).then_some(Ask::Invitation { organization, name, tax }),
        mao_net::n3::OrgAsk::LeavePayTax { organization, tax } => (taxed(tax) && organization != (0, 0)).then_some(Ask::LeavePayTax { organization, tax }),
        mao_net::n3::OrgAsk::TaxChanged { tax } => taxed(tax).then_some(Ask::TaxChanged { tax }),
        mao_net::n3::OrgAsk::Promotion { member } => name_of(member.1).map(|name| Ask::Promotion { member, name }),
    }
}

/// Shows the first waiting question and sends its answer.
pub fn show(ctx: &egui::Context, palette: &crate::theme::Palette) {
    let Some(question) = WAITING.lock().unwrap_or_else(|e| e.into_inner()).front().cloned() else { return };
    if let Some(yes) = crate::chrome::question(ctx, palette, question.window(), &question.title(), &question.text()) {
        WAITING.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
        if let Some((command, at, flag)) = question.answer(yes) {
            crate::net::org_answer(command, at, flag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each box's yes is the original's message, a no sends nothing but for the tax owed, and the server's
    /// questions are only put where the original puts them.
    #[test]
    fn the_boxes_answers_are_the_originals() {
        let organization = (0xdeaa, 77);
        assert_eq!((Ask::Leave.answer(true), Ask::Leave.answer(false)), (Some((0x10, At::Nobody, false)), None));
        assert_eq!((Ask::Disband.answer(true), Ask::Disband.answer(false)), (Some((6, At::Target, false)), None));
        let invitation = Ask::Invitation { organization, name: "Us".into(), tax: 3 };
        assert_eq!((invitation.answer(true), invitation.answer(false)), (Some((0xf, At::Identity(organization), false)), None));
        let owed = Ask::LeavePayTax { organization, tax: 500 };
        assert_eq!((owed.answer(true), owed.answer(false)), (Some((0x1d, At::Identity(organization), false)), Some((0x1e, At::Identity(organization), false))));
        assert_eq!((Ask::TaxChanged { tax: 5 }.answer(true), Ask::TaxChanged { tax: 5 }.answer(false)), (Some((0x10, At::Target, false)), None));
        let promotion = Ask::Promotion { member: (0xc350, 9), name: "Somebody".into() };
        assert_eq!((promotion.answer(true), promotion.answer(false)), (Some((10, At::Identity((0xc350, 9)), true)), None));
        // (the titles the code hands the windows)
        assert_eq!((Ask::Disband.title(), invitation.title(), owed.title(), Ask::TaxChanged { tax: 5 }.title()), ("OrgDisband".into(), "OrgJoin".into(), "OrgLeave".into(), "OrgLeave".into()));
        // (the invitation's text is two texts with two spaces between)
        assert!(invitation.text().contains("  ") && invitation.text().contains("Us"));

        use mao_net::n3::OrgAsk;
        let nobody = |_: u32| None;
        let known = |id: u32| (id == 9).then(|| "Somebody".to_string());
        assert_eq!(from_server(OrgAsk::Invitation { organization, name: "Us".into(), tax: 0 }, nobody), Some(Ask::Invitation { organization, name: "Us".into(), tax: 0 }));
        assert_eq!(from_server(OrgAsk::LeavePayTax { organization, tax: 0 }, nobody), None);
        assert_eq!(from_server(OrgAsk::LeavePayTax { organization, tax: 1_000_000_000 }, nobody), None);
        assert_eq!(from_server(OrgAsk::LeavePayTax { organization, tax: 999_999_999 }, nobody), Some(Ask::LeavePayTax { organization, tax: 999_999_999 }));
        assert_eq!((from_server(OrgAsk::TaxChanged { tax: 0 }, nobody), from_server(OrgAsk::TaxChanged { tax: 7 }, nobody)), (None, Some(Ask::TaxChanged { tax: 7 })));
        assert_eq!(from_server(OrgAsk::Promotion { member: (0xc350, 9) }, nobody), None);
        assert_eq!(from_server(OrgAsk::Promotion { member: (0xc350, 9) }, known), Some(promotion));
    }
}
