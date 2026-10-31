import json
from pathlib import Path

from lindera import Metadata, build_dictionary

# The source of a dictionary with a 4x4 connection matrix, whose
# metadata.json enables connection_id_mapping and whose one entry has the
# context IDs 2, with a histogram that counts 1, 3 and 2 for IDs 1, 2 and 3.
FIXTURE = Path(__file__).resolve().parents[2] / "resources" / "connection_id_mapping"


def _built_context_id_map(output, **kwargs):
    metadata = Metadata.from_json_file(str(FIXTURE / "metadata.json"))
    build_dictionary(str(FIXTURE / "source"), str(output), metadata, **kwargs)
    built = json.loads((output / "metadata.json").read_text(encoding="utf-8"))
    return built["context_id_map"]


def test_build_dictionary_keeps_connection_id_mapping_of_the_metadata(tmp_path):
    # Without a histogram, the IDs other than 0 are ranked by the count of
    # dictionary entries, ties by ID: 2 (the entry's), then 1 and 3.
    assert _built_context_id_map(tmp_path) == {
        "left": [0, 2, 1, 3],
        "right": [0, 2, 1, 3],
    }


def test_build_dictionary_ranks_context_ids_by_the_histogram(tmp_path):
    histogram = str(FIXTURE / "context_id_freq.txt")
    assert _built_context_id_map(tmp_path, context_id_freq=histogram) == {
        "left": [0, 3, 1, 2],
        "right": [0, 3, 1, 2],
    }
