//! In-process rtnetlink helper for bringing netdevs up/down.
//!
//! Most Wi-Fi drivers refuse `NL80211_CMD_SET_INTERFACE` while the netdev is
//! UP and return `EBUSY`. We can't shell out to `ip link` because subprocess
//! capabilities don't survive an exec to a non-cap'd binary — the user's
//! `setcap cap_net_admin,cap_net_raw=eip wifie-server` only covers our
//! process. So we drive `RTM_SETLINK` ourselves.

use futures::stream::TryStreamExt;
use rtnetlink::{new_connection, Handle};

use super::{HwError, HwResult};

pub struct LinkController {
    handle: Handle,
}

impl LinkController {
    pub fn connect() -> HwResult<Self> {
        let (conn, handle, _) = new_connection().map_err(|e| {
            HwError::Other(format!("failed to open rtnetlink socket: {e}"))
        })?;
        tokio::spawn(conn);
        Ok(Self { handle })
    }

    pub async fn set_link_down(&self, name: &str) -> HwResult<()> {
        let if_index = self.lookup_index(name).await?;
        self.handle
            .link()
            .set(if_index)
            .down()
            .execute()
            .await
            .map_err(translate)
    }

    pub async fn set_link_up(&self, name: &str) -> HwResult<()> {
        let if_index = self.lookup_index(name).await?;
        self.handle
            .link()
            .set(if_index)
            .up()
            .execute()
            .await
            .map_err(translate)
    }

    async fn lookup_index(&self, name: &str) -> HwResult<u32> {
        let mut links = self
            .handle
            .link()
            .get()
            .match_name(name.to_string())
            .execute();
        match links.try_next().await {
            Ok(Some(link)) => Ok(link.header.index),
            Ok(None) => Err(HwError::InterfaceNotFound(name.to_string())),
            Err(e) => Err(translate(e)),
        }
    }
}

fn translate(err: rtnetlink::Error) -> HwError {
    let msg = err.to_string();
    let lower = msg.to_ascii_lowercase();
    if lower.contains("eperm") || lower.contains("eacces") || lower.contains("permission") {
        HwError::PermissionDenied
    } else if lower.contains("enodev") || lower.contains("no such device") {
        HwError::InterfaceNotFound(msg)
    } else {
        HwError::Other(msg)
    }
}
