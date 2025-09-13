from flask import Flask, Response, render_template, request
import json

from app.breadcrumb import Redis10List

# Load the JSON data once when the app starts
with open('../work/pretre_jean_forteresse_alamuth.json', 'r', encoding='utf-8') as f:
    data = json.load(f)


def get_data_by_number(number_str):
    """
    Retrieve text and numbers data for a given number string.

    Args:
        number_str (str): The number string to search for

    Returns:
        tuple: (success, data_dict) where:
            - success (bool): Whether the data was found
            - data_dict (dict): Contains 'text', 'numbers', and optionally 'error_message'
    """
    if number_str in data:
        text = ''.join(data[number_str]['text']).split('.')
        numbers = data[number_str]['numbers']
        return True, {
            'text': text,
            'numbers': numbers
        }
    else:
        return False, {
            'error_message': f"le numéro {number_str} n'a pas été trouvé"
        }


def get_text(number_str):
    rl = Redis10List()
    rl.add(number_str)

    # Get data using the factorized function
    success, result_data = get_data_by_number(number_str)

    # Check if it's an AJAX request
    if request.headers.get('Content-Type') == 'application/json' or request.args.get('json'):
        if success:
            return result_data
        else:
            response_data = {
                'success': False,
                'message': result_data['error_message']
            }
            response = Response(
                json.dumps(response_data, ensure_ascii=False, indent=2),
                content_type='application/json; charset=utf-8'
            )
            return response

    # Serve HTML page
    template_data = {
        'current_number': number_str,
        'found': success
    }

    if success:
        template_data.update({
            'text': result_data['text'],
            'numbers': result_data['numbers'],
            'breadcrumb': rl.get_list()

        })
    else:
        template_data['error_message'] = result_data['error_message']

    return render_template('text_display.html', **template_data)
