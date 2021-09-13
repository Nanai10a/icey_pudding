use std::collections::HashSet;

use anyhow::{bail, Result};
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::entities::User;
// FIXME: move to interactors::
use crate::handlers::helpers::*;
use crate::repositories::UserRepository;
use crate::usecases::user::{
    bookmark, edit, get, get_bookmark, gets, register, unbookmark, unregister,
};
use crate::utils::LetChain;

pub struct ReturnUserRegisterInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<register::Output>>,
}
#[async_trait]
impl register::Usecase for ReturnUserRegisterInteractor {
    async fn handle(&self, register::Input { user_id }: register::Input) -> Result<()> {
        let new_user = User {
            id: user_id,
            admin: false,
            sub_admin: false,
            bookmark: HashSet::new(),
        };

        let can_insert = self.user_repository.insert(new_user.clone()).await?;

        if !can_insert {
            bail!("already registered.");
        }

        *self.ret.lock().await = register::Output { user: new_user }.let_(Some);

        Ok(())
    }
}

pub struct ReturnUserGetInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<get::Output>>,
}
#[async_trait]
impl get::Usecase for ReturnUserGetInteractor {
    async fn handle(&self, get::Input { user_id }: get::Input) -> Result<()> {
        *self.ret.lock().await = self
            .user_repository
            .find(user_id)
            .await
            .map_err(user_err_fmt)?
            .let_(|user| get::Output { user })
            .let_(Some);

        Ok(())
    }
}

pub struct ReturnUserGetsInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<gets::Output>>,
}
#[async_trait]
impl gets::Usecase for ReturnUserGetsInteractor {
    async fn handle(&self, gets::Input { query }: gets::Input) -> Result<()> {
        *self.ret.lock().await = self
            .user_repository
            .finds(query)
            .await
            .map_err(user_err_fmt)?
            .let_(|users| gets::Output { users })
            .let_(Some);

        Ok(())
    }
}

pub struct ReturnUserEditInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<edit::Output>>,
}
#[async_trait]
impl edit::Usecase for ReturnUserEditInteractor {
    async fn handle(&self, edit::Input { user_id, mutation }: edit::Input) -> Result<()> {
        *self.ret.lock().await = self
            .user_repository
            .update(user_id, mutation)
            .await
            .map_err(user_err_fmt)?
            .let_(|user| edit::Output { user })
            .let_(Some);

        Ok(())
    }
}

pub struct ReturnUserUnregisterInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<unregister::Output>>,
}
#[async_trait]
impl unregister::Usecase for ReturnUserUnregisterInteractor {
    async fn handle(&self, unregister::Input { user_id }: unregister::Input) -> Result<()> {
        *self.ret.lock().await = self
            .user_repository
            .delete(user_id)
            .await
            .map_err(content_err_fmt)?
            .let_(|user| unregister::Output { user })
            .let_(Some);

        Ok(())
    }
}

pub struct ReturnUserBookmarkGetInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<get_bookmark::Output>>,
}
#[async_trait]
impl get_bookmark::Usecase for ReturnUserBookmarkGetInteractor {
    async fn handle(&self, get_bookmark::Input { user_id }: get_bookmark::Input) -> Result<()> {
        *self.ret.lock().await = self
            .user_repository
            .get_bookmark(user_id)
            .await
            .map_err(content_err_fmt)?
            .let_(|bookmark| get_bookmark::Output { bookmark })
            .let_(Some);

        Ok(())
    }
}

pub struct ReturnUserBookmarkInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<bookmark::Output>>,
}
#[async_trait]
impl bookmark::Usecase for ReturnUserBookmarkInteractor {
    async fn handle(
        &self,
        bookmark::Input {
            user_id,
            content_id,
        }: bookmark::Input,
    ) -> Result<()> {
        let can_insert = self
            .user_repository
            .insert_bookmark(user_id, content_id)
            .await
            .map_err(user_err_fmt)?;

        if !can_insert {
            bail!("already bookmarked.");
        }

        *self.ret.lock().await = self
            .user_repository
            .find(user_id)
            .await
            .map_err(user_err_fmt)?
            .let_(|user| bookmark::Output { user })
            .let_(Some);

        Ok(())
    }
}

pub struct ReturnUserUnbookmarkInteractor {
    pub user_repository: Box<dyn UserRepository + Sync + Send>,
    pub lock: Mutex<()>,
    pub ret: Mutex<Option<unbookmark::Output>>,
}
#[async_trait]
impl unbookmark::Usecase for ReturnUserUnbookmarkInteractor {
    async fn handle(
        &self,
        unbookmark::Input {
            user_id,
            content_id,
        }: unbookmark::Input,
    ) -> Result<()> {
        let can_insert = self
            .user_repository
            .delete_bookmark(user_id, content_id)
            .await
            .map_err(user_err_fmt)?;

        if !can_insert {
            bail!("didn't bookmarked.");
        }

        *self.ret.lock().await = self
            .user_repository
            .find(user_id)
            .await
            .map_err(user_err_fmt)?
            .let_(|user| unbookmark::Output { user })
            .let_(Some);

        Ok(())
    }
}
