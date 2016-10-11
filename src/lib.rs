extern crate iron;
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;

use iron::prelude::*;
use iron::{typemap, BeforeMiddleware};

use std::any::Any;
use std::error::Error;
use std::sync::Arc;
use diesel::{Connection};

/// Iron middleware that allows for diesel postgres connections within requests.
pub struct DieselMiddleware<T> where T: Connection + Send + Any {
  /// A pool of diesel postgres connections that are shared between requests.
  pub pool: Arc<r2d2::Pool<r2d2_diesel::ConnectionManager<T>>>
}

impl<T> typemap::Key for DieselMiddleware<T> where T: Connection + Send + Any {
    type Value = Arc<r2d2::Pool<r2d2_diesel::ConnectionManager<T>>>;
}

impl<T> DieselMiddleware<T> where T: Connection + Send + Any {

    /// Creates a new pooled connection to the given server.
    /// Returns `Err(err)` if there are any errors connecting to the database.
    pub fn new(connection_str: &str) -> Result<DieselMiddleware<T>, Box<Error>> {
        let config = r2d2::Config::default();
        let manager = r2d2_diesel::ConnectionManager::<T>::new(connection_str);
        let pool = try!(r2d2::Pool::new(config, manager));

        Ok(DieselMiddleware {
          pool: Arc::new(pool),
        })
    }
}

impl<T> BeforeMiddleware for DieselMiddleware<T> where T: Connection + Send + Any {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DieselMiddleware<T>>(self.pool.clone());
        Ok(())
    }
}

/// Adds a method to requests to get a database connection.
///
/// ## Example
///
/// ```ignore
/// use iron_diesel_middleware::DieselReqExt;
///
/// fn handler(req: &mut Request) -> IronResult<Response> {
///   let connection = req.db_conn();
///
///   let new_user = NewUser::new("John Smith", 25);
///   diesel::insert(&new_user).into(users::table).execute(&*connection);
///
///   Ok(Response::with((status::Ok, "Added User")))
/// }
/// ```
pub trait DieselReqExt<T> where T: Connection + Send + Any {
  /// Returns a pooled connection to the postgresql database. The connection is returned to
  /// the pool when the pooled connection is dropped.
  ///
  /// **Panics** if a `DieselMiddleware` has not been registered with Iron, or if retrieving
  /// a connection to the database times out.
  fn db_conn(&self) -> r2d2::PooledConnection<r2d2_diesel::ConnectionManager<T>>;
}

impl<'a, 'b, T> DieselReqExt<T> for Request<'a, 'b> where T: Connection + Send + Any {
  fn db_conn(&self) -> r2d2::PooledConnection<r2d2_diesel::ConnectionManager<T>> {
    let poll_value = self.extensions.get::<DieselMiddleware<T>>().unwrap();
    return poll_value.get().unwrap();
  }
}
