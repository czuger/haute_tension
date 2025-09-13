import json
import fnmatch
import argparse
import re
import sys


def search_patterns_in_json(input_file, output_file, patterns):
    """
    Search for wildcard patterns in JSON text data.

    Args:
        input_file (str): Path to input JSON file
        output_file (str): Path to output JSON file
        patterns (list): List of wildcard patterns to search for
    """
    try:
        # Load JSON data
        with open(input_file, 'r', encoding='utf-8') as f:
            data = json.load(f)

        results = {}

        # Compile regex patterns for efficiency
        compiled_patterns = []
        for pattern in patterns:
            try:
                compiled_patterns.append((pattern, re.compile(pattern, re.IGNORECASE)))
            except re.error as e:
                print(f"Error: Invalid regex pattern '{pattern}': {e}")
                sys.exit(1)

        # Iterate through each entry in the JSON
        for key, entry in data.items():
            if 'text' in entry:
                # Join all text elements into one string for searching
                full_text = ' '.join(entry['text'])

                matches = []

                # Search for each pattern
                for pattern_str, compiled_pattern in compiled_patterns:
                    # Find all matches for this pattern
                    found_matches = compiled_pattern.findall(full_text)

                    if found_matches:
                        # Remove duplicates while preserving case
                        unique_matches = list(dict.fromkeys(found_matches))
                        matches.append({
                            'pattern': pattern_str,
                            'matches': unique_matches,
                            'count': len(found_matches)
                        })

                # If any matches found, add to results
                if matches:
                    results[key] = {
                        'entry_key': key,
                        'text': entry['text'],  # Only include text from original entry
                        'pattern_matches': matches,
                        'total_matches': sum(match['count'] for match in matches)
                    }

        # Save results to output file
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)

        print(f"Search completed. Found matches in {len(results)} entries.")
        print(f"Results saved to {output_file}")

        # Print summary
        for key, result in results.items():
            print(f"\nEntry {result['entry_key']}:")
            for match in result['pattern_matches']:
                print(f"  Pattern '{match['pattern']}': {match['matches']} ({match['count']} occurrences)")

    except FileNotFoundError:
        print(f"Error: Input file '{input_file}' not found.")
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Error: Invalid JSON format in '{input_file}'.")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {str(e)}")
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description='Search for wildcard patterns in JSON text data')
    parser.add_argument('input_file', help='Input JSON file path')
    parser.add_argument('output_file', help='Output JSON file path')
    parser.add_argument('patterns', nargs='+', help='Wildcard patterns to search for (e.g., "*ration*" "*provision*")')

    args = parser.parse_args()

    search_patterns_in_json(args.input_file, args.output_file, args.patterns)


if __name__ == "__main__":
    main()
