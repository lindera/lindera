const fs = require("fs");
const https = require("https");
const path = require("path");
const { execSync } = require("child_process");
const { version, buildDictionary, Metadata } = require("lindera-nodejs");

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https
      .get(
        url,
        {
          headers: { "User-Agent": `lindera-nodejs/${version()}` },
        },
        (response) => {
          // Handle redirects
          if (response.statusCode === 301 || response.statusCode === 302) {
            download(response.headers.location, dest)
              .then(resolve)
              .catch(reject);
            return;
          }
          response.pipe(file);
          file.on("finish", () => {
            file.close(resolve);
          });
        },
      )
      .on("error", (err) => {
        fs.unlinkSync(dest);
        reject(err);
      });
  });
}

async function main() {
  const url = "https://lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz";
  const filename = "/tmp/mecab-ipadic-2.7.0-20070801.tar.gz";

  // Download dictionary source file
  console.log("Downloading dictionary source...");
  await download(url, filename);

  // Extract the dictionary source file
  console.log("Extracting...");
  execSync(`tar xzf ${filename} -C /tmp/`);

  const sourcePath = "/tmp/mecab-ipadic-2.7.0-20070801";
  const destinationPath = "/tmp/lindera-ipadic-2.7.0-20070801";
  const metadataPath = path.resolve(
    __dirname,
    "../resources/ipadic_metadata.json",
  );

  const metadata = Metadata.fromJsonFile(metadataPath);

  // Build dictionary
  console.log("Building dictionary...");
  buildDictionary(sourcePath, destinationPath, metadata);

  // List all files in the destination directory
  console.log(`\nFiles created in ${destinationPath}:`);
  const files = fs.readdirSync(destinationPath);
  for (const file of files) {
    const filePath = path.join(destinationPath, file);
    const stats = fs.statSync(filePath);
    console.log(`  ${file} (${stats.size.toLocaleString()} bytes)`);
  }
  console.log();
}

main().catch(console.error);
