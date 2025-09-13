from flask import Flask, jsonify, Response, send_file
import json

from flask import redirect
from flask import url_for

from app.get_text import get_text

app = Flask(__name__)


def get_oldest_page():
    filename = "last_pages.json"

    try:
        with open(filename, 'r') as f:
            last_pages = json.load(f)

        if last_pages:  # Check if list is not empty
            return last_pages[0]  # Return first (oldest) element
        else:
            return None  # Return None if list is empty

    except FileNotFoundError:
        return "1"


def update_last_pages(current_page: str):
    filename = "last_pages.json"

    # Try to load existing file, or initialize with empty list
    try:
        with open(filename, 'r') as f:
            last_pages = json.load(f)
    except FileNotFoundError:
        last_pages = []

    # Append current page
    last_pages.append(current_page)

    # Keep only last 10 elements
    if len(last_pages) > 10:
        last_pages = last_pages[-10:]

    # Save back to file
    with open(filename, 'w') as f:
        json.dump(last_pages, f)

    return last_pages

@app.route('/')
def index():
    # Redirect to a default number (replace 'default_number' with actual number)
    return redirect(url_for('get_text', number_str='1'))


app.route('/text/<string:number_str>')(get_text)

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5001, debug=True)