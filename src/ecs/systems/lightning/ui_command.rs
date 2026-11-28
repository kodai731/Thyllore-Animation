#[derive(Clone, Debug)]
pub enum LightningUiCommand {
    AddTarget,
    ClearTarget,
    AddWaypoint,
    RemoveWaypoint(usize),
}
