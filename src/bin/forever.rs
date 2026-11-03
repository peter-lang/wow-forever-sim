//! `forever druid-level <command> ...`: the druid's levelling routes (`druid::beam::cli`).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("druid-level") {
        forever::druid::beam::cli(&args[1..]);
    } else {
        eprintln!("usage: forever druid-level <command> ... (README)");
        std::process::exit(2);
    }
}
