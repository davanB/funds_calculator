use transactions::{
    parse_transactions, process_transactions, read_transaction_file, write_client_funds,
};

fn main() {
    if let Err(error) = read_transaction_file()
        .and_then(parse_transactions)
        .and_then(process_transactions)
        .and_then(write_client_funds)
    {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
