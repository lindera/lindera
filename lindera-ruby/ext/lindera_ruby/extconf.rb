require "rb_sys/mkmf"

create_rust_makefile("lindera/lindera_ruby") do |r|
  # Pass embed features via LINDERA_FEATURES environment variable
  # e.g., LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
  features = ENV.fetch("LINDERA_FEATURES", "").split(",").map(&:strip).reject(&:empty?)
  r.features = features unless features.empty?
end
