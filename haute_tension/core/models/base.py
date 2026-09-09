"""Turning a document into a plain dict, and back.

The app is written against dicts — `load_story()` hands them out, the routes read
them, the templates render them — so the models are a boundary, not a type that
travels. Every model mixes this in, and `core.db` converts on the way in and on
the way out. The dicts are the `TypedDict`s in
`haute_tension.application.models.story_page`, which is what makes a stored page
and an imported `merged_pages.json` page read the same.

Two rules the rest of the package leans on:

- **The dict is keyed as the document is stored**, so a stored page and the raw
  imported payload have the same keys. The one exception is the primary key: a
  model whose id is the book's own (a page number) hands it over under its
  attribute name, which is what the app calls it and what Mongo keeps in `_id`;
  a generated ObjectId is Mongo's own bookkeeping and never leaves here.
- **`to_dict()` always returns every declared field**, `None` where nothing is
  stored, so a caller can read a key without knowing whether it was ever
  written; **`from_dict()` ignores anything undeclared**, which is what lets a
  raw imported page be handed over as it came.
"""

from typing import Any

from mongoengine import ObjectIdField
from mongoengine.base import BaseDocument


def plain(value: object) -> Any:
    """Return a document's value as plain Python.

    Args:
        value: Any value read off a document.

    Returns:
        The same value with no `BaseList`, `BaseDict` or embedded document left
        in it.
    """
    if isinstance(value, BaseDocument):
        # Every document in this package mixes in DictDocument, so this is a
        # to_dict() call and not a mongoengine one.
        return value.to_dict()  # type: ignore[attr-defined]
    if isinstance(value, (list, tuple)):
        return [plain(item) for item in value]
    if isinstance(value, dict):
        return {key: plain(item) for key, item in value.items()}
    return value


class DictDocument:
    """A document that goes in and comes out as a dict keyed like the stored one."""

    @classmethod
    def stored_fields(cls) -> dict[str, str]:
        """Return `{dict key: attribute name}`, in declaration order.

        A declared primary key is the book's own id: it is kept, under its
        attribute name rather than under `_id`. The ObjectId mongoengine adds
        when a model declares no key of its own is Mongo's bookkeeping, and
        stays out of the dict.

        Returns:
            Every dict key this model reads and writes, mapped to its attribute.
        """
        primary_key = cls._meta.get("id_field")
        fields: dict[str, str] = {}
        for name in cls._fields_ordered:
            field = cls._fields[name]
            if name != primary_key:
                fields[field.db_field] = name
            elif not isinstance(field, ObjectIdField):
                fields[name] = name
        return fields

    @classmethod
    def collection_name(cls) -> str:
        """Return the collection this model is stored in."""
        return cls._get_collection_name()

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "DictDocument":
        """Build a document from a stored-shaped dict, ignoring undeclared keys.

        Args:
            data: A dict keyed the way this model is stored.

        Returns:
            The unsaved document.
        """
        fields = cls.stored_fields()
        return cls(
            **{fields[key]: value for key, value in data.items() if key in fields}
        )

    def to_dict(self) -> dict[str, Any]:
        """Return every declared field, `None` where unset, keyed as stored."""
        return {
            key: plain(getattr(self, name))
            for key, name in self.stored_fields().items()
        }
