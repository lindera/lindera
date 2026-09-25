# Builds the extension for both `bundle exec rake compile` and `gem install`
# (the gemspec lists this file as the extension). Cargo.toml sits next to it in
# both layouts, so rb_sys finds the crate without further configuration.
require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("lindera/lindera_ruby") do |r|
  # Pass embed features via LINDERA_FEATURES environment variable, e.g.
  #   LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
  #   LINDERA_FEATURES="embed-ipadic" gem install lindera
  features = ENV.fetch("LINDERA_FEATURES", "").split(",").map(&:strip).reject(&:empty?)
  r.features = features unless features.empty?
end
