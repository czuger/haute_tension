import json
import openai
import uuid


def create_batch(input_path: str):
    with open('config.json', 'r') as f:
        config = json.load(f)

    # Charger les prompts depuis le fichier JSON
    with open('prompts.json', 'r', encoding='utf-8') as f:
        prompts = json.load(f)

    client = openai.OpenAI(api_key=config['api_key'])

    with open(input_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # Créer les requêtes batch
    batch_requests = []

    for section_id, section_data in data.items():
        if 'text' in section_data:
            full_text = ' '.join(section_data['text'])

            # Construire le contenu utilisateur en ajoutant full_text à la fin
            user_content = prompts['user_prompt'] + full_text

            request = {
                "custom_id": f"section_{section_id}",
                "method": "POST",
                "url": "/v1/chat/completions",
                "body": {
                    "model": "gpt-5-nano",
                    "messages": [
                        {
                            "role": "system",
                            "content": prompts['system_prompt']
                        },
                        {
                            "role": "user",
                            "content": user_content
                        }
                    ]
                }
            }
            batch_requests.append(request)

    print(f"Nombre de requêtes à traiter: {len(batch_requests)}")

    # Vérifier les limites
    if len(batch_requests) > 50000:
        print("ERREUR: Trop de requêtes (limite: 50,000)")
        return None

    # Créer le fichier batch
    batch_filename = f"batch_requests_{uuid.uuid4().hex[:8]}.jsonl"
    with open(batch_filename, 'w', encoding='utf-8') as f:
        for request in batch_requests:
            f.write(json.dumps(request) + '\n')

    # Uploader le fichier
    print(f"Upload du fichier {batch_filename}...")
    with open(batch_filename, 'rb') as f:
        file_response = client.files.create(
            file=f,
            purpose='batch'
        )

    # Créer le batch
    print("Création du batch...")
    batch = client.batches.create(
        input_file_id=file_response.id,
        endpoint="/v1/chat/completions",
        completion_window="24h"
    )

    # Sauvegarder les infos du batch
    batch_info = {
        "batch_id": batch.id,
        "file_id": file_response.id,
        "status": batch.status,
        "created_at": batch.created_at,
        "request_counts": batch.request_counts.__dict__ if batch.request_counts else None
    }

    with open(f"batch_info_{batch.id}.json", 'w') as f:
        json.dump(batch_info, f, indent=2)

    print(f"Batch créé avec succès!")
    print(f"Batch ID: {batch.id}")
    print(f"Status: {batch.status}")
    print(f"Infos sauvées dans: batch_info_{batch.id}.json")

    return batch.id


if __name__ == "__main__":
    INPUT_FILE = "pretre_jean_forteresse_alamuth.json"
    batch_id = create_batch(INPUT_FILE)
