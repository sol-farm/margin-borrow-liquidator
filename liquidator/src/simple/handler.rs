use super::*;
use db::models::Obligation;
impl SimpleLiquidator {
    pub fn handle_liquidation_check(
        self: &Arc<Self>,
        obligation: &Obligation,
    ) -> Result<()> {

        Ok(())
    }
}