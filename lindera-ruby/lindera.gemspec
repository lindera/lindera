# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "lindera"
  spec.version = "2.3.4"
  spec.authors = ["Lindera contributors"]
  spec.summary = "Ruby bindings for Lindera morphological analysis engine"
  spec.description = "Ruby bindings for Lindera, a morphological analysis library for CJK text (Japanese, Korean, Chinese)."
  spec.homepage = "https://github.com/lindera/lindera"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1"

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rs,toml,rb,lock}",
    "Cargo.*",
    "README.md",
    "LICENSE",
  ]
  spec.extensions = ["ext/lindera_ruby/extconf.rb"]
  spec.require_paths = ["lib"]

  spec.add_dependency "rb_sys", "~> 0.9"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
end
