import hashlib
import json
from pathlib import Path

import openai
import pygame
import io
import tempfile
import os

# Set your OpenAI API key
openai.api_key = "your-api-key-here"  # Replace with your actual API key


def get_text_hash(text):
    """Generate hash for text"""
    return hashlib.sha256(text.encode('utf-8')).hexdigest()

def get_audio_filename(text_hash):
    """Get the audio filename for a given hash"""
    return Path("speeches") / f"{text_hash}.mp3"

def speak_french_text(text, voice="onyx", model="gpt-4o-mini-tts"):
    """
    Convert French text to speech using OpenAI and play it on macOS

    Args:
        text (str): The French text to speak
        voice (str): Voice to use ("alloy", "echo", "fable", "onyx", "nova", "shimmer")
        model (str): TTS model ("tts-1" or "tts-1-hd" for higher quality)
    """
    text_hash = get_text_hash(text)
    audio_file = get_audio_filename(text_hash)

    if not audio_file.exists():
        with open('../work/analyse/config.json', 'r') as config_file:
            config = json.load(config_file)

        # Create OpenAI client
        client = openai.OpenAI(api_key=config['open_ai']['openai_key'])

        # Generate speech
        response = client.audio.speech.create(
            model=model,
            voice=voice,
            input=text,
            instructions="Read this text in the style of an epic heroic fantasy narrator. Use a deep, resonant, dramatic voice. Distinguish characters by slightly adjusting your tone: the hero is earnest and bold, the villain is sly and dark, and the old wizard is wise and gentle. Emphasize fantasy names and magical spells, pacing narration to create both excitement in battles and awe in descriptions."
        )

        # Save to permanent file
        with open(audio_file, 'wb') as f:
            f.write(response.content)

    # # Initialize pygame mixer
    # pygame.mixer.init()
    #
    # # Load and play the audio
    # pygame.mixer.music.load(audio_file)
    # pygame.mixer.music.play()
    #
    # # Wait for the audio to finish playing
    # while pygame.mixer.music.get_busy():
    #     pygame.time.wait(100)
    #
    # # Clean up
    # pygame.mixer.quit()

    print("Audio playback completed!")

# Example usage
if __name__ == "__main__":
    french_text = """
    Comment allez-vous lutter contre ce Démon du Styx ?Vous vous jetez sur lui l'épée au poing. 40 Vous lui parlez en vers. 21 Vous pensez qu'il s'agit d'une illusion et vous attendez. 461 Si le Génie de l'oasis vous a fait un don. 590"
    """
    speak_french_text(french_text, voice="onyx")
