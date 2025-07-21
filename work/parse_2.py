import yaml
from bs4 import BeautifulSoup
import re
import json

with open('raw_data/pretre_jean_forteresse_alamuth.yaml', 'r') as f:
    data = yaml.safe_load(f)

result = {}

for url, info in data.items():
    file_path = info[':file_path']

    # Extract number from URL
    url_match = re.search(r'/(\d+)', url)
    url_number = url_match.group(1) if url_match else url

    with open(file_path, 'r', encoding='utf-8') as html_file:
        soup = BeautifulSoup(html_file.read(), 'html.parser')

        sections = soup.find_all('div', class_='ob-sections')
        texts = []
        numbers = []

        for section in sections:
            text_divs = section.find_all('div', class_='ob-text')
            for text_div in text_divs:
                # Get the HTML content first to process links
                html_content = str(text_div)

                # Find all links and replace them with "link_text number"
                links = text_div.find_all('a', href=True)
                for link in links:
                    href = link['href']
                    link_text = link.get_text().strip()
                    match = re.search(r'/(\d+)', href)
                    if match:
                        number = match.group(1)
                        # Replace the entire link tag with "link_text number"
                        replacement = f"{link_text} {number} "
                        html_content = html_content.replace(str(link), replacement)

                # Parse the modified HTML and get text
                modified_soup = BeautifulSoup(html_content, 'html.parser')
                text = modified_soup.get_text().strip()

                if text:  # Only add non-empty texts
                    texts.append(text)

                if number:  # Only add non-empty texts
                    numbers.append(number)


        result[url_number] = {
            "text": texts,
            "numbers": numbers,
            "file_path": file_path
        }

# Save to JSON file with beautiful formatting
with open('pretre_jean_forteresse_alamuth.json', 'w', encoding='utf-8') as f:
    json.dump(result, f, ensure_ascii=False, indent=2)

print("Data saved to pretre_jean_forteresse_alamuth.json")
