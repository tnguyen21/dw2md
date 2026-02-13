# This is a placeholder formula. It will be auto-updated by the release CI
# when a new version is tagged. See .github/workflows/release.yml for details.
#
# To use this tap:
#   brew tap tnguyen21/dw2md
#   brew install dw2md

class Dw2md < Formula
  desc "Crawl a DeepWiki repository and compile all pages into markdown"
  homepage "https://github.com/tnguyen21/dw2md"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tnguyen21/dw2md/releases/download/v0.2.0/dw2md-macos-aarch64"
      sha256 "PLACEHOLDER_AARCH64_SHA256"
    else
      url "https://github.com/tnguyen21/dw2md/releases/download/v0.2.0/dw2md-macos-x86_64"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    url "https://github.com/tnguyen21/dw2md/releases/download/v0.2.0/dw2md-linux-x86_64-musl"
    sha256 "PLACEHOLDER_LINUX_MUSL_SHA256"
  end

  def install
    bin.install stable.url.split("/").last => "dw2md"
  end

  test do
    assert_match "dw2md", shell_output("#{bin}/dw2md --version")
  end
end
