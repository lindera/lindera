# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "lindera"
  spec.version = "6.2.0"
  spec.authors = ["Lindera contributors"]
  spec.summary = "Ruby bindings for Lindera morphological analysis engine"
  spec.description = "Ruby bindings for Lindera, a morphological analysis library for CJK text (Japanese, Korean, Chinese)."
  spec.homepage = "https://github.com/lindera/lindera"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1"

  # A source gem: `gem install` compiles the crate with extconf.rb. Build it
  # with `bundle exec rake build`, which packages from the crate as
  # `cargo package` normalizes it. Running `gem build` here would ship this
  # directory's Cargo.toml, which only resolves inside the Cargo workspace.
  spec.files = Dir[
    "Cargo.toml",
    "LICENSE",
    "README.md",
    "extconf.rb",
    "lib/**/*.rb",
    "src/**/*.rs",
  ]
  spec.extensions = ["extconf.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "rb_sys", "~> 0.9"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
end
