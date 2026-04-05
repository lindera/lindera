import os
import tarfile
import urllib.request

from lindera import Metadata, build_dictionary, version


def main():
    url = "https://lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz"
    filename = "/tmp/mecab-ipadic-2.7.0-20070801.tar.gz"

    # Add User-Agent header to avoid 403 error
    opener = urllib.request.build_opener()
    opener.addheaders = [("User-Agent", f"lindera-python/{version()}")]
    urllib.request.install_opener(opener)

    # Download dictionary source file
    urllib.request.urlretrieve(url, filename)

    # Extract the dictionary source file
    with tarfile.open(filename, "r:gz") as tar:
        tar.extractall("/tmp/", filter="data")

    source_path = "/tmp/mecab-ipadic-2.7.0-20070801"
    destination_path = "/tmp/lindera-ipadic-2.7.0-20070801"
    metadata_path = "./resources/ipadic_metadata.json"

    metadata = Metadata.from_json_file(metadata_path)

    # Build dictionary
    build_dictionary(source_path, destination_path, metadata)

    # List all files in the destination directory
    print(f"\nFiles created in {destination_path}:")
    for root, dirs, files in os.walk(destination_path):
        for file in files:
            file_path = os.path.join(root, file)
            rel_path = os.path.relpath(file_path, destination_path)
            file_size = os.path.getsize(file_path)
            print(f"  {rel_path} ({file_size:,} bytes)")
    print()


if __name__ == "__main__":
    main()
