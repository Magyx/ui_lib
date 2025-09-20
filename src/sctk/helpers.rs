use smithay_client_toolkit::output::OutputState;

use super::OutputSelector;

pub(super) fn pick_output<'a>(
    outputs: &OutputState,
    sel: &OutputSelector<'a>,
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
