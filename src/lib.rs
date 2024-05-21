use std::{ops::Deref, sync::Arc};

use concurrent_queue::{ConcurrentQueue, PushError};
use onetime::{channel, RecvError, Sender};
use thiserror::Error;

#[derive(Debug)]
struct Waiters<T>(ConcurrentQueue<Sender<T>>);

impl<T> Default for Waiters<T> {
	fn default() -> Self {
		Self(ConcurrentQueue::unbounded())
	}
}

impl<T> Deref for Waiters<T> {
	type Target = ConcurrentQueue<Sender<T>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Customer can request resource from the linked [Vendor]
#[derive(Debug, Clone)]
pub struct Customer<T> {
	waiters: Arc<Waiters<T>>,
}

impl<T> Customer<T> {
	/// Queuing up for a resource
	///
	/// # Errors
	/// You may get an error if you failed to queue or take a resource.
	pub async fn request(&self) -> Result<T, RequestError> {
		let (tx, rx) = channel();
		self.waiters.push(tx)?;
		rx.recv().await.map_err(Into::into)
	}
}

/// Vendor can send the resource to waiting [Customer]s.
/// If there is no waiting [Customer]s, resource will be lost
#[derive(Debug)]
pub struct Vendor<T> {
	waiters: Arc<Waiters<T>>,
}

impl<T> Default for Vendor<T> {
	fn default() -> Self {
		Self { waiters: Arc::default() }
	}
}

impl<T> Vendor<T> {
	pub fn new() -> Self {
		Self::default()
	}

	/// Creates a customer linked to this [Vendor]
	pub fn customer(&self) -> Customer<T> {
		Customer { waiters: self.waiters.clone() }
	}

	/// Sends the resource to customers
	///
	/// Note, if no one is in the queue the resource will be lost
	pub fn send(&self, resource: T)
	where
		T: Clone,
	{
		while let Ok(waiter) = self.waiters.pop() {
			let _ = waiter.send(resource.clone());
		}
	}

	pub fn waiters_count(&self) -> usize {
		self.waiters.len()
	}

	/// It may be useful to check the queue for waiting customers to do something
	/// only if someone is in the queue
	pub fn has_waiters(&self) -> bool {
		self.waiters_count() > 0
	}
}

#[derive(Debug, Error)]
#[error("failed to request resource")]
pub enum RequestError {
	Push,
	Recv,
}

impl<T> From<PushError<T>> for RequestError {
	fn from(_: PushError<T>) -> Self {
		Self::Push
	}
}

impl From<RecvError> for RequestError {
	fn from(_: RecvError) -> Self {
		Self::Recv
	}
}

#[derive(Debug, Error)]
#[error("failed to send resource")]
pub struct SendError;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
		smol::block_on(async move {
			let vendor = Vendor::new();
			let customer1 = vendor.customer();
			let customer2 = vendor.customer();

			let t1 = smol::spawn(async move {
				assert!(matches!(customer1.request().await, Ok("ok")));
			});

			let t2 = smol::spawn(async move {
				assert!(matches!(customer2.request().await, Ok("ok")));
			});

			let t3 = smol::spawn(async move {
				vendor.send("ok");
			});

			t1.await;
			t2.await;
			t3.await;
		});
	}
}
