@app.route('/audio/<int:number>')
def get_audio(number):
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