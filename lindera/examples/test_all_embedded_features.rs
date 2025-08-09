fn main() {
    println!("=== Embedded Dictionary Feature Status ===");

    // IPADIC
    println!(
        "embedded-ipadic feature: {}",
        cfg!(feature = "embedded-ipadic")
    );
    println!("ipadic feature: {}", cfg!(feature = "ipadic"));

    // UniDic
    println!(
        "embedded-unidic feature: {}",
        cfg!(feature = "embedded-unidic")
    );
    println!("unidic feature: {}", cfg!(feature = "unidic"));

    // Ko-Dic
    println!(
        "embedded-ko-dic feature: {}",
        cfg!(feature = "embedded-ko-dic")
    );
    println!("ko-dic feature: {}", cfg!(feature = "ko-dic"));

    // CC-CEDICT
    println!(
        "embedded-cc-cedict feature: {}",
        cfg!(feature = "embedded-cc-cedict")
    );
    println!("cc-cedict feature: {}", cfg!(feature = "cc-cedict"));

    // IPADIC-NEologd
    println!(
        "embedded-ipadic-neologd feature: {}",
        cfg!(feature = "embedded-ipadic-neologd")
    );
    println!(
        "ipadic-neologd feature: {}",
        cfg!(feature = "ipadic-neologd")
    );

    println!("\n=== Dictionary Variants ===");
    let variants = lindera::dictionary::DictionaryKind::contained_variants();
    println!("contained_variants: {:?}", variants);
    println!("contained_variants count: {}", variants.len());

    println!("\n=== Available Dictionaries ===");
    for variant in variants {
        println!("- {}", variant.as_str());
    }
}
