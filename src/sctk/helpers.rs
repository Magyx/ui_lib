use smithay_client_toolkit::{
    output::OutputState, reexports::client::protocol::wl_output::WlOutput, seat::keyboard::Keysym,
};
use smol_str::ToSmolStr;

use crate::{event::LogicalKey, sctk::OutputSet};

use super::OutputSelector;

pub(super) enum PickedOutput {
    Active,
    Output(WlOutput),
}

impl PickedOutput {
    pub fn as_wl_output(&self) -> Option<&WlOutput> {
        match self {
            PickedOutput::Active => None,
            PickedOutput::Output(o) => Some(o),
        }
    }

    pub fn into_option(self) -> Option<WlOutput> {
        match self {
            PickedOutput::Active => None,
            PickedOutput::Output(o) => Some(o),
        }
    }
}

pub(super) enum OutputPickError {
    NoOutputs,
    NoMatch,
}

pub(super) fn pick_output(
    outputs: &OutputState,
    sel: &OutputSelector,
) -> Result<WlOutput, OutputPickError> {
    use OutputSelector::*;

    let out = match sel {
        First => outputs.outputs().next(),
        Index(i) => outputs.outputs().nth(*i),
        NamePrefix(prefix) => outputs.outputs().find(|o| {
            outputs.info(o).is_some_and(|info| {
                info.name.as_deref().unwrap_or_default().starts_with(prefix)
                    || info.model.starts_with(prefix)
                    || info.make.starts_with(prefix)
            })
        }),
        InternalPrefer => {
            let is_internal = |o: &_| {
                outputs.info(o).is_some_and(|info| {
                    let n = info.name.as_deref().unwrap_or_default();
                    n.starts_with("eDP") || n.starts_with("LVDS")
                })
            };
            outputs
                .outputs()
                .find(is_internal)
                .or_else(|| outputs.outputs().next())
        }
        HighestScale => outputs
            .outputs()
            .max_by_key(|o| outputs.info(o).map(|i| i.scale_factor).unwrap_or(1)),
    };

    out.ok_or_else(|| {
        if outputs.outputs().next().is_none() {
            OutputPickError::NoOutputs
        } else {
            OutputPickError::NoMatch
        }
    })
}

pub(super) fn pick_outputs(outputs: &OutputState, set: &OutputSet) -> Vec<PickedOutput> {
    use OutputSet::*;

    match set {
        Active => vec![PickedOutput::Active],

        All => {
            let outs: Vec<_> = outputs.outputs().map(PickedOutput::Output).collect();
            if outs.is_empty() {
                vec![PickedOutput::Active]
            } else {
                outs
            }
        }

        One(sel) => match pick_output(outputs, sel) {
            Ok(o) => vec![PickedOutput::Output(o)],
            Err(_) => vec![PickedOutput::Active],
        },

        List(list) => {
            let outs: Vec<_> = list
                .iter()
                .filter_map(|s| pick_output(outputs, s).ok())
                .map(PickedOutput::Output)
                .collect();

            if outs.is_empty() {
                vec![PickedOutput::Active]
            } else {
                outs
            }
        }
    }
}

pub(super) fn map_keysym_to_logical(k: Keysym, utf8: Option<&str>) -> LogicalKey {
    use smithay_client_toolkit::seat::keyboard::Keysym as KS;
    match k {
        KS::Return | KS::KP_Enter | KS::ISO_Enter | KS::Linefeed => LogicalKey::Enter,
        KS::Escape => LogicalKey::Escape,
        KS::BackSpace => LogicalKey::Backspace,
        KS::Tab | KS::ISO_Left_Tab | KS::KP_Tab => LogicalKey::Tab,
        KS::space | KS::KP_Space => LogicalKey::Space,

        KS::Left | KS::KP_Left => LogicalKey::ArrowLeft,
        KS::Right | KS::KP_Right => LogicalKey::ArrowRight,
        KS::Up | KS::KP_Up => LogicalKey::ArrowUp,
        KS::Down | KS::KP_Down => LogicalKey::ArrowDown,
        KS::Home | KS::KP_Home => LogicalKey::Home,
        KS::End | KS::KP_End => LogicalKey::End,
        KS::Page_Up | KS::KP_Page_Up => LogicalKey::PageUp,
        KS::Page_Down | KS::KP_Page_Down => LogicalKey::PageDown,
        KS::Insert | KS::KP_Insert => LogicalKey::Insert,
        KS::Delete | KS::KP_Delete => LogicalKey::Delete,

        KS::Shift_L | KS::Shift_R => LogicalKey::Shift,
        KS::Control_L | KS::Control_R => LogicalKey::Control,
        KS::Alt_L | KS::Alt_R => LogicalKey::Alt,
        KS::ISO_Level3_Shift | KS::Mode_switch => LogicalKey::AltGraph,
        KS::Super_L | KS::Super_R | KS::Meta_L | KS::Meta_R => LogicalKey::Super,
        KS::Caps_Lock => LogicalKey::CapsLock,
        KS::Num_Lock => LogicalKey::NumLock,
        KS::Menu => LogicalKey::ContextMenu,

        other if other.is_function_key() => {
            let n = other.raw() - KS::F1.raw() + 1;
            LogicalKey::F(n as u8)
        }

        other if (KS::dead_grave.raw()..=0xfe93).contains(&other.raw()) => LogicalKey::Dead,

        _ => utf8
            .map(|s| LogicalKey::Character(s.to_smolstr()))
            .unwrap_or(LogicalKey::Unknown),
    }
}
