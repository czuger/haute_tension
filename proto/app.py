from flask import Flask, jsonify, Response, send_file
import json

from proto.speak import speak_french_text, get_text_hash, get_audio_filename

app = Flask(__name__)

# Load the JSON data once when the app starts
with open('pretre_jean_forteresse_alamuth.json', 'r', encoding='utf-8') as f:
    data = json.load(f)


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


@app.route('/data/<int:number>')
def get_numbers(number):
    number_str = str(number)

    _data = data[number_str]
    _data["numbers"].sort()

    return jsonify(_data)


@app.route('/audio/<int:number>')
def get_text(number):
    number_str = str(number)

    print(number)

    update_last_pages(number_str)

    if number_str in data:
        # Join all text entries with newlines
        text = '\n'.join(data[number_str]['text'])

        speak_french_text(text)

        # Get the filename
        text_hash = get_text_hash(text)
        audio_file = get_audio_filename(text_hash)

        return send_file(
            audio_file,
            mimetype='audio/mpeg',
            as_attachment=True,
            download_name=f'text_{number}.mp3')
    else:
        print("not found")
        response_data = {
            'success': False,
            'message': f"le numéro {number} n'a pas été trouvé"
        }

    # Create response with proper UTF-8 encoding
    response = Response(
        json.dumps(response_data, ensure_ascii=False, indent=2),
        content_type='application/json; charset=utf-8'
    )
    return response


if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5001, debug=True)