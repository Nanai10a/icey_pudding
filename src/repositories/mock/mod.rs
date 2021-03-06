use core::ops::RangeBounds;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::{ContentRepository, RepositoryError, Result, UserRepository};
use crate::entities::{Author, Content, ContentId, User, UserId};
use crate::usecases::content::{
    AuthorQuery, ContentContentMutation, ContentMutation, ContentQuery, PostedQuery,
};
use crate::usecases::user::{UserMutation, UserQuery};

mod helpers;

use helpers::*;

pub struct InMemoryRepository<T>(Mutex<Vec<T>>);

impl<T> InMemoryRepository<T> {
    pub fn new() -> Self { Self(Mutex::new(vec![])) }
}
impl<T> Default for InMemoryRepository<T> {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl UserRepository for InMemoryRepository<User> {
    async fn insert(&self, item: User) -> Result<bool> {
        let mut guard = self.0.lock().await;

        match find_ref(&guard, |v| v.id == item.id) {
            Ok(_) => return Ok(false),
            Err(RepositoryError::NotFound) => (),
            Err(e) => return Err(e),
        }

        tracing::trace!("insert - {:?}", item);

        guard.push(item);
        Ok(true)
    }

    async fn is_exists(&self, id: UserId) -> Result<bool> {
        let guard = self.0.lock().await;

        match find_ref(&guard, |v| v.id == id) {
            Ok(_) => Ok(true),
            Err(RepositoryError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn find(&self, id: UserId) -> Result<User> {
        let guard = self.0.lock().await;

        Ok(find_ref(&guard, |v| v.id == id)?.clone())
    }

    #[tracing::instrument(skip(self))]
    async fn finds(
        &self,
        UserQuery {
            bookmark,
            bookmark_num,
        }: UserQuery,
    ) -> Result<Vec<User>> {
        let res = self
            .0
            .lock()
            .await
            .iter()
            .filter(|u| {
                bookmark
                    .as_ref()
                    .map(|s| s.is_subset(&u.bookmark))
                    .unwrap_or(true)
            })
            .filter(|u| {
                bookmark_num
                    .as_ref()
                    .map(|b| b.contains(&(u.bookmark.len() as u32)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        tracing::trace!("found - {:?}", res);

        Ok(res)
    }

    #[tracing::instrument(skip(self))]
    async fn update(
        &self,
        id: UserId,
        UserMutation { admin, sub_admin }: UserMutation,
    ) -> Result<User> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |v| v.id == id)?;

        tracing::trace!("found - {:?}", item);

        if let Some(val) = admin {
            item.admin = val;
        }
        if let Some(val) = sub_admin {
            item.sub_admin = val;
        }

        tracing::trace!("mutated - {:?}", item);

        Ok(item.clone())
    }

    async fn get_bookmark(&self, id: UserId) -> Result<std::collections::HashSet<ContentId>> {
        let User { bookmark, .. } = self.find(id).await?;

        Ok(bookmark)
    }

    async fn is_bookmark(&self, id: UserId, content_id: ContentId) -> Result<bool> {
        let guard = self.0.lock().await;
        let User { bookmark, .. } = find_ref(&guard, |u| u.id == id)?;

        match bookmark.iter().filter(|v| **v == content_id).count() {
            0 => Ok(false),
            1 => Ok(true),
            i => Err(RepositoryError::NoUnique { matched: i as u32 }),
        }
    }

    async fn insert_bookmark(&self, id: UserId, content_id: ContentId) -> Result<bool> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |u| u.id == id)?;

        Ok(item.bookmark.insert(content_id))
    }

    async fn delete_bookmark(&self, id: UserId, content_id: ContentId) -> Result<bool> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |u| u.id == id)?;

        Ok(item.bookmark.remove(&content_id))
    }

    #[tracing::instrument(skip(self))]
    async fn delete(&self, id: UserId) -> Result<User> {
        let mut guard = self.0.lock().await;
        let mut res = guard
            .iter()
            .enumerate()
            .filter(|(_, v)| v.id == id)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        tracing::trace!("found - {:?}", res);

        let index = match res.len() {
            0 => return Err(RepositoryError::NotFound),
            1 => res.remove(0),
            i => return Err(RepositoryError::NoUnique { matched: i as u32 }),
        };

        Ok(guard.remove(index))
    }
}

#[async_trait]
impl ContentRepository for InMemoryRepository<Content> {
    async fn insert(&self, item: Content) -> Result<bool> {
        let mut guard = self.0.lock().await;

        match find_ref(&guard, |v| v.id == item.id) {
            Ok(_) => return Ok(false),
            Err(RepositoryError::NotFound) => (),
            Err(e) => return Err(e),
        }

        tracing::trace!("insert - {:?}", item);

        guard.push(item);
        Ok(true)
    }

    async fn is_exists(&self, id: ContentId) -> Result<bool> {
        let guard = self.0.lock().await;

        match find_ref(&guard, |v| v.id == id) {
            Ok(_) => Ok(true),
            Err(RepositoryError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn find(&self, id: ContentId) -> Result<Content> {
        let guard = self.0.lock().await;

        Ok(find_ref(&guard, |v| v.id == id)?.clone())
    }

    #[tracing::instrument(skip(self))]
    async fn finds(
        &self,
        ContentQuery {
            author,
            posted,
            content,
            liked,
            liked_num,
            pinned,
            pinned_num,
        }: ContentQuery,
    ) -> Result<Vec<Content>> {
        let res = self
            .00
            .lock()
            .await
            .iter()
            .filter(|c| {
                author
                    .as_ref()
                    .map(|q| match &c.author {
                        Author::User { id, name, nick } => match q {
                            AuthorQuery::UserId(q_id) => q_id == id,
                            AuthorQuery::UserName(q_r) => q_r.is_match(name.as_str()),
                            AuthorQuery::UserNick(q_r) => nick
                                .as_ref()
                                .map(|n| q_r.is_match(n.as_str()))
                                .unwrap_or(false),
                            AuthorQuery::Any(q_r) =>
                                (q_r.is_match(name.as_str())
                                    || nick
                                        .as_ref()
                                        .map(|n| q_r.is_match(n.as_str()))
                                        .unwrap_or(false)),
                            _ => false,
                        },
                        Author::Virtual(name) => match q {
                            AuthorQuery::Virtual(q_r) => q_r.is_match(name.as_str()),
                            AuthorQuery::Any(q_r) => q_r.is_match(name.as_str()),
                            _ => false,
                        },
                    })
                    .unwrap_or(true)
            })
            .filter(|c| {
                posted
                    .as_ref()
                    .map(|q| match q {
                        PostedQuery::UserId(q_id) => *q_id == c.posted.id,
                        PostedQuery::UserName(q_r) => q_r.is_match(c.posted.name.as_str()),
                        PostedQuery::UserNick(q_r) => c
                            .posted
                            .nick
                            .as_ref()
                            .map(|n| q_r.is_match(n.as_str()))
                            .unwrap_or(false),
                        PostedQuery::Any(q_r) =>
                            (q_r.is_match(c.posted.name.as_str())
                                || c.posted
                                    .nick
                                    .as_ref()
                                    .map(|n| q_r.is_match(n.as_str()))
                                    .unwrap_or(false)),
                    })
                    .unwrap_or(true)
            })
            .filter(|c| {
                content
                    .as_ref()
                    .map(|r| r.is_match(c.content.as_str()))
                    .unwrap_or(true)
            })
            .filter(|c| {
                liked
                    .as_ref()
                    .map(|s| s.is_subset(&c.liked))
                    .unwrap_or(true)
            })
            .filter(|c| {
                liked_num
                    .as_ref()
                    .map(|b| b.contains(&(c.liked.len() as u32)))
                    .unwrap_or(true)
            })
            .filter(|c| {
                pinned
                    .as_ref()
                    .map(|s| s.is_subset(&c.pinned))
                    .unwrap_or(true)
            })
            .filter(|c| {
                pinned_num
                    .as_ref()
                    .map(|b| b.contains(&(c.pinned.len() as u32)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        tracing::trace!("found - {:?}", res);

        Ok(res)
    }

    #[tracing::instrument(skip(self))]
    async fn update(
        &self,
        id: ContentId,
        ContentMutation {
            author,
            content,
            edited,
        }: ContentMutation,
    ) -> Result<Content> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |c| c.id == id)?;

        tracing::trace!("found - {:?}", item);

        if let Some(new_author) = author {
            item.author = new_author;
        }
        match content {
            Some(ContentContentMutation::Complete(new_content)) => {
                item.content = new_content;
            },
            Some(ContentContentMutation::Sed { capture, replace }) => {
                item.content = capture.replace(item.content.as_ref(), replace).to_string();
            },
            None => (),
        };

        item.edited.push(edited);

        tracing::trace!("mutated - {:?}", item);

        Ok(item.clone())
    }

    async fn get_liked(&self, id: ContentId) -> Result<std::collections::HashSet<UserId>> {
        let Content { liked, .. } = self.find(id).await?;

        Ok(liked)
    }

    async fn is_liked(&self, id: ContentId, user_id: UserId) -> Result<bool> {
        let guard = self.0.lock().await;
        let item = find_ref(&guard, |c| c.id == id)?;

        match item.liked.iter().filter(|v| **v == user_id).count() {
            0 => Ok(false),
            1 => Ok(true),
            i => Err(RepositoryError::NoUnique { matched: i as u32 }),
        }
    }

    async fn insert_liked(&self, id: ContentId, user_id: UserId) -> Result<bool> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |c| c.id == id)?;

        Ok(item.liked.insert(user_id))
    }

    async fn delete_liked(&self, id: ContentId, user_id: UserId) -> Result<bool> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |c| c.id == id)?;

        Ok(item.liked.remove(&user_id))
    }

    async fn get_pinned(&self, id: ContentId) -> Result<std::collections::HashSet<UserId>> {
        let Content { pinned, .. } = self.find(id).await?;

        Ok(pinned)
    }

    async fn is_pinned(&self, id: ContentId, user_id: UserId) -> Result<bool> {
        let guard = self.0.lock().await;
        let item = find_ref(&guard, |c| c.id == id)?;

        match item.pinned.iter().filter(|v| **v == user_id).count() {
            0 => Ok(false),
            1 => Ok(true),
            i => Err(RepositoryError::NoUnique { matched: i as u32 }),
        }
    }

    async fn insert_pinned(&self, id: ContentId, user_id: UserId) -> Result<bool> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |c| c.id == id)?;

        Ok(item.pinned.insert(user_id))
    }

    async fn delete_pinned(&self, id: ContentId, user_id: UserId) -> Result<bool> {
        let mut guard = self.0.lock().await;
        let item = find_mut(&mut guard, |c| c.id == id)?;

        Ok(item.pinned.remove(&user_id))
    }

    async fn delete(&self, id: ContentId) -> Result<Content> {
        let mut guard = self.0.lock().await;
        let mut res = guard
            .iter()
            .enumerate()
            .filter(|(_, v)| v.id == id)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        tracing::trace!("found - {:?}", res);

        let index = match res.len() {
            0 => return Err(RepositoryError::NotFound),
            1 => res.remove(0),
            i => return Err(RepositoryError::NoUnique { matched: i as u32 }),
        };

        Ok(guard.remove(index))
    }
}
