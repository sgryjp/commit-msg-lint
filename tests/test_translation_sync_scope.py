import json
from pathlib import Path
import unittest


class TranslationSyncScopeTests(unittest.TestCase):
    def test_agent_instructions_are_excluded_from_translation_review(self):
        root = Path(__file__).resolve().parent.parent
        config = json.loads((root / "docs/translation-sync.json").read_text(encoding="utf-8"))
        for pair in config["pairs"]:
            with self.subTest(pair=pair["id"]):
                for language in ("en", "ja"):
                    self.assertFalse(pair[language].startswith("docs/agents/"))


if __name__ == "__main__":
    unittest.main()
