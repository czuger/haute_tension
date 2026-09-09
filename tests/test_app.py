import unittest
from unittest.mock import patch

from haute_tension import app as app_module


class AppEntryPointTestCase(unittest.TestCase):
    """Tests for the development-server entry point."""

    def test_module_exposes_a_configured_application(self) -> None:
        """Build the application from the packaged book at import time."""
        self.assertEqual(
            sorted(app_module.app.blueprints),
            ["api", "web"],
        )

    def test_main_serves_on_localhost_only(self) -> None:
        """Serve on the loopback interface so the debugger stays off the LAN."""
        with patch.object(app_module.app, "run") as run:
            app_module.main()

        run.assert_called_once_with(host="127.0.0.1", port=5001, debug=True)


if __name__ == "__main__":
    unittest.main()
