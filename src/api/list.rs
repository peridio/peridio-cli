use clap::Args;

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Limit the length of the page.
    #[arg(long)]
    pub limit: Option<u8>,
    /// Specify whether the query is ordered ascending or descending.
    #[arg(long, value_enum)]
    pub order: Option<String>,
    /// A search query per the Peridio API's search query language. It is recommended to quote the value of this option.
    #[arg(long)]
    pub search: String,
    /// A cursor for pagination across multiple pages of results. Don't include this parameter on the first call. Use the next_page value returned in a previous response (if not null) to request subsequent results.
    #[arg(long)]
    pub page: Option<String>,
}
