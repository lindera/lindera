use lindera::LinderaResult;
use lindera::dictionary::DictionaryKind;
use lindera_cli::get_version;

#[derive(Debug, clap::Args)]
#[clap(
    author,
    about = "List embedded morphological analysis dictionaries",
    version = get_version(),
)]
pub struct ListArgs {}

pub fn list(_args: ListArgs) -> LinderaResult<()> {
    for dic in DictionaryKind::contained_variants() {
        println!("{}", dic.as_str());
    }
    Ok(())
}
