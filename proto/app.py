from flask import Flask, jsonify, Response, send_file
import json

from proto.speak import speak_french_text, get_text_hash, get_audio_filename

app = Flask(__name__)

# Load the JSON data once when the app starts
with open('pretre_jean_forteresse_alamuth.json', 'r', encoding='utf-8') as f:
    data = json.load(f)


@app.route('/<int:number>')
def get_text(number):
    number_str = str(number)

    print(number)

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