from haute_tension.application.factory import create_app

app = create_app()


def main() -> None:
    """Run the Flask development server."""
    app.run(host="0.0.0.0", port=5001, debug=True)


if __name__ == "__main__":
    main()
