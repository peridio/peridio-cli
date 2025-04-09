use peridio_sdk::{api::ApiOptions, list_params::ListParams, Api};

use crate::GlobalOptions;

use super::list::ListArgs;

pub trait ApiExt {
    fn from_options(options: GlobalOptions) -> Self;
}

impl ApiExt for Api {
    fn from_options(options: GlobalOptions) -> Self {
        Api::new(ApiOptions {
            api_key: options.api_key.unwrap(),
            endpoint: options.base_url,
            ca_bundle_path: options.ca_path,
        })
    }
}

pub trait ListExt {
    fn from_args(args: &ListArgs) -> Self;
}

impl ListExt for ListParams {
    fn from_args(args: &ListArgs) -> Self {
        ListParams {
            limit: args.limit,
            order: args.order.clone(),
            search: args.search.clone(),
            page: args.page.clone(),
        }
    }
}
