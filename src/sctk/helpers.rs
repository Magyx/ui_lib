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
        KS::Return => LogicalKey::Enter,
        KS::Escape => LogicalKey::Escape,
        KS::BackSpace => LogicalKey::Backspace,
        KS::Tab => LogicalKey::Tab,
        KS::space => LogicalKey::Space,
        KS::Left => LogicalKey::ArrowLeft,
        KS::Right => LogicalKey::ArrowRight,
        KS::Up => LogicalKey::ArrowUp,
        KS::Down => LogicalKey::ArrowDown,
        KS::Home => LogicalKey::Home,
        KS::End => LogicalKey::End,
        KS::Page_Up => LogicalKey::PageUp,
        KS::Page_Down => LogicalKey::PageDown,
        KS::Insert => LogicalKey::Insert,
        KS::Delete => LogicalKey::Delete,
        _ => utf8
            .map(|s| LogicalKey::Character(s.to_smolstr()))
            .unwrap_or(LogicalKey::Unknown),
    }
}
