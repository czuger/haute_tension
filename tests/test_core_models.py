"""Tests for the dict boundary the models are."""

from haute_tension.core.models import PageView, StoryChoice, StoryPage
from haute_tension.core.models.base import plain


class TestStoredFields:
    """Which keys a model reads and writes."""

    def test_a_source_id_is_kept_under_its_own_name(self):
        assert StoryPage.stored_fields()["page"] == "page"

    def test_a_generated_object_id_stays_out(self):
        assert "id" not in PageView.stored_fields()
        assert "_id" not in PageView.stored_fields()

    def test_a_renamed_field_is_keyed_as_stored(self):
        assert StoryPage.stored_fields()["file_path"] == "file_path"

    def test_the_collection_is_named(self):
        assert StoryPage.collection_name() == "story_pages"
        assert PageView.collection_name() == "page_views"


class TestToDict:
    """What comes out of a document."""

    def test_every_declared_field_is_present(self):
        assert StoryPage(page="1", book="b").to_dict() == {
            "page": "1",
            "book": "b",
            "language": None,
            "text": [],
            "file_path": None,
            "choices": [],
            "fight": {},
        }

    def test_embedded_documents_come_out_as_dicts(self):
        page = StoryPage.from_dict(
            {
                "page": "1",
                "choices": [
                    {"goto": "2", "gains": [{"element": "sword", "amount": 1}]}
                ],
            }
        )

        choice = page.to_dict()["choices"][0]
        assert isinstance(choice, dict)
        assert choice["gains"] == [{"element": "sword", "amount": 1}]


class TestFromDict:
    """What a document accepts."""

    def test_undeclared_keys_are_ignored(self):
        page = StoryPage.from_dict({"page": "1", "scraped_at": "yesterday"})

        assert page.page == "1"
        assert "scraped_at" not in page.to_dict()

    def test_a_page_without_choices_gets_none(self):
        assert StoryPage.from_dict({"page": "1"}).to_dict()["choices"] == []

    def test_a_null_choices_list_is_tolerated(self):
        assert StoryPage.from_dict({"page": "1", "choices": None}).choices == []

    def test_a_malformed_choice_is_dropped(self):
        page = StoryPage.from_dict({"page": "1", "choices": ["2", {"goto": "3"}]})

        assert [choice.goto for choice in page.choices] == ["3"]

    def test_a_choice_without_element_changes_gets_none(self):
        choice = StoryChoice.from_dict({"goto": "2"})

        assert choice.gains == []
        assert choice.losses == []

    def test_a_malformed_element_change_is_dropped(self):
        choice = StoryChoice.from_dict({"goto": "2", "gains": ["sword"]})

        assert choice.gains == []

    def test_element_changes_that_are_not_a_list_are_dropped(self):
        choice = StoryChoice.from_dict({"goto": "2", "gains": {"element": "sword"}})

        assert choice.gains == []


class TestPlain:
    """Turning a stored value into plain Python."""

    def test_a_scalar_is_itself(self):
        assert plain("1") == "1"
        assert plain(None) is None

    def test_a_tuple_becomes_a_list(self):
        assert plain(("1", "2")) == ["1", "2"]

    def test_nesting_is_followed(self):
        assert plain({"a": [{"b": ("c",)}]}) == {"a": [{"b": ["c"]}]}


class TestStr:
    """What a document prints as, for a message about a run."""

    def test_a_page_names_its_book_and_number(self):
        assert str(StoryPage(page="22", book="serie/livre")) == "serie/livre p.22"

    def test_a_choice_names_its_destination(self):
        assert str(StoryChoice(goto="621")) == "-> 621"

    def test_a_view_names_what_was_read(self):
        view = PageView(book="serie/livre", page="22", viewed_at="now")

        assert str(view) == "serie/livre p.22 @ now"
