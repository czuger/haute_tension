"""One numbered page of a book, with its choices and its encounter.

The document mirrors the `StoryPage` TypedDict the app is written against, so a
page read from Mongo and a page read from an imported `merged_pages.json` are
the same dict. The book's own page number is the primary key: it is what a
choice's `goto` points at and what every route looks a page up by.

`fight` is stored as a free-form dict rather than an embedded document. Its shape
varies with `fight_type` and the import pipeline is still moving it around; the
app only ever hands the whole object to a template, so declaring it field by
field would buy nothing and break on the next enrichment pass.
"""

from mongoengine import (
    DictField,
    Document,
    EmbeddedDocument,
    EmbeddedDocumentListField,
    IntField,
    ListField,
    StringField,
)

from .base import DictDocument


class ElementChange(EmbeddedDocument, DictDocument):
    """An inventory or status change attached to a choice."""

    meta = {"strict": False}

    element = StringField(required=True)
    amount = IntField()


class StoryChoice(EmbeddedDocument, DictDocument):
    """A link from one page to another, with what it costs and what it gives."""

    meta = {"strict": False}

    goto = StringField(required=True)
    gains = EmbeddedDocumentListField(ElementChange)
    losses = EmbeddedDocumentListField(ElementChange)

    @classmethod
    def from_dict(cls, data: dict[str, object]) -> "StoryChoice":
        """Build a choice with its gains and losses turned into documents.

        Args:
            data: One choice as the imported book stores it.

        Returns:
            The unsaved embedded choice.
        """
        choice = super().from_dict(
            {
                key: value
                for key, value in data.items()
                if key not in ("gains", "losses")
            }
        )
        choice.gains = _element_changes(data.get("gains"))
        choice.losses = _element_changes(data.get("losses"))
        return choice

    def __str__(self) -> str:
        return f"-> {self.goto}"


class StoryPage(Document, DictDocument):
    """A numbered page as the last import stored it."""

    meta = {"collection": "story_pages", "strict": False}

    # The book's own page number, kept as the primary key: it is what a choice's
    # `goto` points at, and it is a string everywhere in the app.
    page = StringField(primary_key=True)

    # The book this page belongs to, as "<series>/<book>". Every read filters on
    # it, so a second imported book never leaks into the first one's pages.
    book = StringField(required=True)

    language = StringField()
    text = ListField(StringField())
    file_path = StringField()
    choices = EmbeddedDocumentListField(StoryChoice)
    fight = DictField()

    @classmethod
    def from_dict(cls, data: dict[str, object]) -> "StoryPage":
        """Build a page with its choices turned into embedded documents.

        Args:
            data: One page as `merged_pages.json` stores it.

        Returns:
            The unsaved page.
        """
        page = super().from_dict(
            {key: value for key, value in data.items() if key != "choices"}
        )
        raw_choices = data.get("choices") or []
        page.choices = [
            StoryChoice.from_dict(choice)
            for choice in raw_choices
            if isinstance(choice, dict)
        ]
        return page

    def __str__(self) -> str:
        return f"{self.book} p.{self.page}"


def _element_changes(changes: object) -> list[ElementChange]:
    """Turn a choice's stored gains or losses into embedded documents.

    Args:
        changes: The raw list, or anything falsy when the choice has none.

    Returns:
        One embedded document per well-formed entry.
    """
    if not isinstance(changes, list):
        return []
    return [
        ElementChange.from_dict(change)
        for change in changes
        if isinstance(change, dict)
    ]
