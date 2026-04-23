Pod::Spec.new do |spec|
  spec.name         = "IrohLibFramework"
  spec.version      = "0.98.1"
  spec.summary      = "Compiled Rust library for Iroh"
  spec.description  = <<-DESC
                   Compiled Rust library for Iroh - a toolkit for building distributed applications.
                   DESC
  spec.homepage     = "https://github.com/kylehowells/iroh-ffi"
  spec.license      = { :type => "MIT & Apache License, Version 2.0",   :text => <<-LICENSE
                          Refer to LICENSE-MIT and LICENSE-APACHE in the repository.
                        LICENSE
                      }
  spec.author       = { "Kyle Howells" => "" }
  spec.ios.deployment_target  = '15.0'
  spec.osx.deployment_target  = '12.0'
  spec.static_framework = true
  spec.source = { :http => "https://github.com/kylehowells/iroh-ffi/releases/download/v#{spec.version}/Iroh-ios.xcframework.zip" }
  spec.ios.vendored_frameworks = 'Iroh-ios.xcframework'
end
