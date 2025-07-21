from typing import TypedDict


class ElementChange(TypedDict):
    """An inventory or status change attached to a choice."""

    element: str
    amount: int


class StoryChoice(TypedDict):
    """A link from one story page to another with its state changes."""

    goto: str
    gains: list[ElementChange]
    losses: list[ElementChange]


class RequiredStoryPage(TypedDict):
    """Fields shared by every story page."""

    page: str
    language: str
    text: list[str]
    file_path: str
    choices: list[StoryChoice]


class StoryPage(RequiredStoryPage, total=False):
    """A numbered story page with optional encounter data."""

    fight: dict[str, object]


StoryData = dict[str, StoryPage]
