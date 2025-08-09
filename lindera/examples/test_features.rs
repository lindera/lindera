fn main() {
    println!("ipadic feature: {}", cfg!(feature = "ipadic"));
    println!(
        "embedded-ipadic feature: {}",
        cfg!(feature = "embedded-ipadic")
    );
    println!("compress feature: {}", cfg!(feature = "compress"));
    println!("mmap feature: {}", cfg!(feature = "mmap"));

    let variants = lindera::dictionary::DictionaryKind::contained_variants();
    println!("contained_variants: {variants:?}");
    println!("contained_variants count: {}", variants.len());
}
