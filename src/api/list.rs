use clap::Args;

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, help = "Limit the length of the page.")]
    pub limit: Option<u8>,
    #[arg(
        long,
        value_enum,
        help = "Specify whether the query is ordered ascending or descending."
    )]
    pub order: Option<String>,
    #[arg(
        long,
        help = "A search query per the Peridio API's search query language. It is recommended to quote the value of this option."
    )]
    pub search: String,
    #[arg(
        long,
        help = "A cursor for pagination across multiple pages of results. Don't include this parameter on the first call. Use the next_page value returned in a previous response (if not null) to request subsequent results."
    )]
    pub page: Option<String>,
}
