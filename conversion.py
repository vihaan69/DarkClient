"""
This script downloads the Minecraft client mappings for the latest stable version,
converts them into a custom JSON format, and saves the output to 'mappings.json'.
"""
import re
import json
import requests

def get_latest_minecraft_version():
    """
    Retrieves the latest stable Minecraft version number from the Mojang version manifest.

    Returns:
        str: The latest stable version number, or None if an error occurs.
    """
    try:
        print("Fetching Minecraft version manifest...")
        response = requests.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json", timeout=10)
        response.raise_for_status()  # Raise an exception for bad HTTP responses
        manifest = response.json()
        latest_version = manifest.get("latest", {}).get("release")
        if latest_version:
            print(f"Latest stable version found: {latest_version}")
            return latest_version
        else:
            print("Error: Could not find the latest stable version in the manifest.")
            return None
    except requests.exceptions.RequestException as e:
        print(f"Error while fetching the version manifest: {e}")
        return None
    except json.JSONDecodeError:
        print("Error: Failed to decode JSON response from Mojang's server.")
        return None

def download_client_mappings(version):
    """
    Downloads the client mappings file for a specific Minecraft version.

    Args:
        version (str): The Minecraft version to download the mappings for.

    Returns:
        str: The text content of the mappings file, or None if an error occurs.
    """
    try:
        print(f"Fetching information for version {version}...")
        # First, get the version manifest to find the URL for the specific version details
        response = requests.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json", timeout=10)
        response.raise_for_status()
        manifest = response.json()
        
        version_info_url = next((v['url'] for v in manifest['versions'] if v['id'] == version), None)
        
        if not version_info_url:
            print(f"Error: Could not find the information URL for version {version}.")
            return None

        print(f"Downloading detailed information for version {version}...")
        version_info_response = requests.get(version_info_url, timeout=10)
        version_info_response.raise_for_status()
        version_info = version_info_response.json()

        mappings_url = version_info.get("downloads", {}).get("client_mappings", {}).get("url")
        
        if not mappings_url:
            print(f"Error: Could not find the mappings URL for version {version}.")
            return None

        print(f"Downloading client mappings from {mappings_url}...")
        mappings_response = requests.get(mappings_url, timeout=30)
        mappings_response.raise_for_status()
        print("Mappings download complete.")
        return mappings_response.text
    except requests.exceptions.RequestException as e:
        print(f"Error while downloading mappings: {e}")
        return None
    except json.JSONDecodeError:
        print("Error: Failed to decode JSON response while fetching version details.")
        return None


def convert_java_type_to_jvm(java_type, class_map):
    """Converts a Java type to its internal JVM representation."""
    array_depth = java_type.count("[]")
    java_type = java_type.replace("[]", "")

    primitive_map = {
        "void": "V", "boolean": "Z", "byte": "B", "char": "C",
        "short": "S", "int": "I", "float": "F", "long": "J", "double": "D"
    }

    if java_type in primitive_map:
        jvm_type = primitive_map[java_type]
    else:
        internal_name = java_type.replace('.', '/')
        obfuscated_name = class_map.get(internal_name, internal_name)
        jvm_type = f"L{obfuscated_name};"

    return ("[" * array_depth) + jvm_type

def get_method_signature(return_type, params_str, class_map):
    """Generates a method signature in JVM format."""
    params = []
    if params_str:
        for param in params_str.split(','):
            param_type = param.strip().split(' ')[0]
            params.append(convert_java_type_to_jvm(param_type, class_map))

    return_type_jvm = convert_java_type_to_jvm(return_type, class_map)
    return f"({''.join(params)}){return_type_jvm}"

def parse_mappings(input_text):
    """
    Parses the mappings text and converts it into a Python data structure.

    Args:
        input_text (str): The text content of the ProGuard mappings.

    Returns:
        dict: A dictionary representing the parsed mappings.
    """
    data = {"classes": {}}
    class_map = {}

    class_re = re.compile(r'^([\w\.$]+) -> ([\w$]+):$')
    method_re = re.compile(r'^\s+(?:\d+:\d+:)?([\w\.<>$]+)\s+([\w<>$]+)\((.*)\)\s+->\s+([\w<>$]+)$')
    field_re = re.compile(r'^\s+([\w\.<>$]+)\s+([\w$]+)\s+->\s+([\w$]+)$')

    lines = [line.rstrip() for line in input_text.split('\n') if line.strip() and not line.startswith('#')]

    print("First pass: parsing class definitions...")
    for line in lines:
        if class_match := class_re.match(line):
            original_java = class_match.group(1)
            obfuscated_name = class_match.group(2)
            original_jvm = original_java.replace('.', '/')
            class_map[original_jvm] = obfuscated_name
            data["classes"][original_jvm] = {
                "name": obfuscated_name,
                "methods": {},
                "fields": {}
            }
    print(f"Found and parsed {len(class_map)} classes.")

    print("Second pass: parsing methods and fields...")
    current_class_jvm = None
    for line in lines:
        if class_match := class_re.match(line):
            current_class_jvm = class_match.group(1).replace('.', '/')
            continue

        if not current_class_jvm:
            continue

        if method_match := method_re.match(line):
            return_type, method_name, params, obf_method = method_match.groups()
            signature = get_method_signature(return_type, params, class_map)
            data["classes"][current_class_jvm]["methods"][method_name] = {
                "name": obf_method,
                "signature": signature
            }
        elif field_match := field_re.match(line):
            field_type, field_name, obf_field = field_match.groups()
            data["classes"][current_class_jvm]["fields"][field_name] = {
                "name": obf_field
            }
    print("Method and field parsing complete.")
    return data

def main():
    """
    Main function that orchestrates the download, parsing, and saving of mappings.
    """
    latest_version = get_latest_minecraft_version()
    if not latest_version:
        return

    mappings_text = download_client_mappings(latest_version)
    if not mappings_text:
        return

    print("Converting mappings into the data structure...")
    mapping_data = parse_mappings(mappings_text)

    output_filename = 'mappings.json'
    print(f"Writing output to '{output_filename}'...")
    try:
        with open(output_filename, 'w', encoding='utf-8') as f:
            json.dump(mapping_data, f, indent=4, ensure_ascii=False)
        print("Operation completed successfully!")
        print(f"The file '{output_filename}' has been created/overwritten.")
    except IOError as e:
        print(f"Error while writing the JSON file: {e}")

if __name__ == '__main__':
    main()