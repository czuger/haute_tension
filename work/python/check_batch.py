import json
import openai
import sys


def process_batch_results(batch_id: str, original_input_path: str, output_path: str):
    with open('config.json', 'r') as f:
        config = json.load(f)

    client = openai.OpenAI(api_key=config['api_key'])

    # Vérifier le status du batch
    print(f"Vérification du batch {batch_id}...")
    batch_status = client.batches.retrieve(batch_id)

    print(f"Status: {batch_status.status}")
    print(f"Request counts: {batch_status.request_counts.__dict__ if batch_status.request_counts else 'N/A'}")

    if batch_status.status == "validating":
        print("Le batch est en cours de validation...")
        return False
    elif batch_status.status == "in_progress":
        print("Le batch est en cours de traitement...")
        return False
    elif batch_status.status == "finalizing":
        print("Le batch est en cours de finalisation...")
        return False
    elif batch_status.status == "failed":
        print("Le batch a échoué!")
        if batch_status.errors:
            print("Erreurs:")
            for error in batch_status.errors.data:
                print(f"- {error}")
        return False
    elif batch_status.status == "expired":
        print("Le batch a expiré!")
        return False
    elif batch_status.status == "cancelled":
        print("Le batch a été annulé!")
        return False
    elif batch_status.status != "completed":
        print(f"Status inconnu: {batch_status.status}")
        return False

    print("Batch terminé avec succès!")

    # Récupérer les résultats
    if not batch_status.output_file_id:
        print("Aucun fichier de résultats disponible!")
        return False

    print("Téléchargement des résultats...")
    result_file = client.files.content(batch_status.output_file_id)
    results = result_file.content.decode('utf-8').strip().split('\n')

    # Charger les données originales
    with open(original_input_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # Parser et intégrer les résultats
    success_count = 0
    error_count = 0

    for result_line in results:
        try:
            result = json.loads(result_line)
            custom_id = result['custom_id']
            section_id = custom_id.replace('section_', '')

            if 'error' in result:
                print(f"Erreur pour section {section_id}: {result['error']}")
                data[section_id]['rations'] = []
                error_count += 1
                continue

            content = result['response']['body']['choices'][0]['message']['content']
            objects = json.loads(content.strip())
            if objects != []:
                print(objects)
            data[section_id]['rations'] = objects
            success_count += 1

        except Exception as e:
            print(f"Erreur parsing résultat: {e}")
            error_count += 1
            continue

    # Sauvegarder les résultats
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"\nRésultats:")
    print(f"- Succès: {success_count}")
    print(f"- Erreurs: {error_count}")
    print(f"- Total: {success_count + error_count}")
    print(f"- Résultats sauvés dans: {output_path}")

    return True


if __name__ == "__main__":
    BATCH_ID = "batch_68c30c59a0e48190a09f36a41feaba32"
    INPUT_FILE = "pretre_jean_forteresse_alamuth.json"
    OUTPUT_FILE = "pretre_jean_forteresse_alamuth_w_rations.json"

    success = process_batch_results(BATCH_ID, INPUT_FILE, OUTPUT_FILE)
    if not success:
        print("Le batch n'est pas encore terminé. Réessayez plus tard.")
