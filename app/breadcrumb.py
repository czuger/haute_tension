import redis

REDIS_KEY = "haute-tension"


class Redis10List:
    def __init__(self):
        """
        Initialize Redis connection.
        Uses default connection: localhost:6379, db=0
        Uses constant key: haute-tension
        """
        self.redis_client = redis.Redis(decode_responses=True)

    def add(self, element):
        """
        Add a string element to the list. Maintains exactly 10 elements.
        If list has 10 elements, removes the oldest one before adding the new one.

        Args:
            element (str): String element to add
        """
        # Convert to string to ensure it's a string
        element = str(element)

        # Add element to the right (newest)
        self.redis_client.rpush(REDIS_KEY, element)

        # If list exceeds 10 elements, remove from left (oldest)
        if self.redis_client.llen(REDIS_KEY) > 10:
            self.redis_client.lpop(REDIS_KEY)

    def get_list(self):
        """
        Get the full list of elements.

        Returns:
            list: List of all string elements (oldest to newest)
        """
        return self.redis_client.lrange(REDIS_KEY, 0, -1)

    def first(self):
        """
        Get the first (oldest) element from the list.

        Returns:
            str: First element in the list, or "1" if list doesn't exist or is empty
        """
        first_element = self.redis_client.lindex(REDIS_KEY, 0)
        return first_element if first_element is not None else "1"


# Example usage:
if __name__ == "__main__":
    # Create an instance
    my_list = Redis10List()

    # Test when list is empty
    print("First element (empty list):", my_list.first())  # Should return "1"

    # Add some elements
    for i in range(12):
        my_list.add(f"item_{i}")

    # Get the full list and first element
    print("Full list:", my_list.get_list())
    print("First element:", my_list.first())  # Should return "item_2" (oldest remaining)
