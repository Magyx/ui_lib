use smithay_client_toolkit::{output::OutputState, seat::keyboard::Keysym};
use smol_str::ToSmolStr;

use crate::{event::LogicalKey, sctk::OutputSet};

use super::OutputSelector;

pub(super) fn pick_output(
    outputs: &OutputState,
    sel: &OutputSelector,
) -> Option<wayland_client::protocol::wl_output::WlOutput> {
    use OutputSelector::*;

    match sel {
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
    }
}

pub(super) fn pick_outputs(
    outputs: &OutputState,
    sel: &OutputSet,
) -> Vec<wayland_client::protocol::wl_output::WlOutput> {
    match sel {
        OutputSet::All => outputs.outputs().collect(),
        OutputSet::List(list) => list
            .iter()
            .filter_map(|s| pick_output(outputs, s))
            .collect(),
        OutputSet::One(s) => pick_output(outputs, s).into_iter().collect(),
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
