#!/usr/bin/env python3
"""
Batch Processing Example

Process multiple audio/video files and generate a report with fingerprints,
tags, and similarity scores.

Usage:
    python batch_processing.py /path/to/media/directory
    python batch_processing.py /path/to/media/directory --output report.json
"""

import sys
import json
import argparse
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, List, Any

from kino_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger


SUPPORTED_EXTENSIONS = {'.wav', '.mp3', '.flac', '.ogg', '.m4a', '.mp4', '.mkv', '.webm', '.avi'}


def process_file(file_path: Path) -> Dict[str, Any]:
    """Process a single media file and return analysis results."""

    analyzer = FrequencyAnalyzer(sample_rate=44100)
    fingerprinter = Fingerprinter()
    tagger = ContentTagger()

    result = {
        "file": str(file_path),
        "name": file_path.name,
        "success": False,
        "error": None,
    }

    try:
        # Frequency analysis
        analysis = analyzer.analyze(str(file_path))
        result["dominant_frequencies"] = [
            {"frequency": f.frequency, "magnitude": f.magnitude}
            for f in analysis.dominant_frequencies[:5]
        ]
        result["spectral_centroid"] = analysis.spectral_centroid
        result["spectral_rolloff"] = analysis.spectral_rolloff
        result["spectral_flatness"] = analysis.spectral_flatness

        # Fingerprint
        fingerprint = fingerprinter.fingerprint(str(file_path))
        result["fingerprint"] = {
            "hash": fingerprint.hash,
            "duration_secs": fingerprint.duration_secs,
            "peak_count": fingerprint.peak_count,
        }

        # Tags
        tags = tagger.predict(str(file_path))
        result["tags"] = [
            {"name": t.name, "category": t.category, "confidence": t.confidence}
            for t in tags
        ]

        # Signature for similarity
        signature = analyzer.compute_signature(str(file_path))
        result["signature"] = {
            "mel_bands": signature.mel_bands,
            "mfcc": signature.mfcc,
        }

        result["success"] = True

    except Exception as e:
        result["error"] = str(e)

    return result


def find_similar_content(results: List[Dict[str, Any]], threshold: float = 0.7) -> List[Dict[str, Any]]:
    """Find similar content pairs based on frequency signatures."""

    similar_pairs = []

    for i, r1 in enumerate(results):
        if not r1["success"] or "signature" not in r1:
            continue

        for j, r2 in enumerate(results[i+1:], i+1):
            if not r2["success"] or "signature" not in r2:
                continue

            # Compute cosine similarity between mel bands
            mel1 = r1["signature"]["mel_bands"]
            mel2 = r2["signature"]["mel_bands"]

            if len(mel1) == len(mel2) and len(mel1) > 0:
                dot = sum(a * b for a, b in zip(mel1, mel2))
                norm1 = sum(a * a for a in mel1) ** 0.5
                norm2 = sum(a * a for a in mel2) ** 0.5

                if norm1 > 0 and norm2 > 0:
                    similarity = dot / (norm1 * norm2)

                    if similarity >= threshold:
                        similar_pairs.append({
                            "file1": r1["name"],
                            "file2": r2["name"],
                            "similarity": round(similarity, 4),
                        })

    return sorted(similar_pairs, key=lambda x: -x["similarity"])


def main():
    parser = argparse.ArgumentParser(description="Batch process media files for frequency analysis")
    parser.add_argument("directory", help="Directory containing media files")
    parser.add_argument("--output", "-o", help="Output JSON file for results")
    parser.add_argument("--workers", "-w", type=int, default=4, help="Number of parallel workers")
    parser.add_argument("--similarity-threshold", "-t", type=float, default=0.7,
                        help="Similarity threshold for finding duplicates (0.0-1.0)")
    args = parser.parse_args()

    directory = Path(args.directory)
    if not directory.is_dir():
        print(f"Error: {directory} is not a directory")
        sys.exit(1)

    # Find all supported media files
    files = [f for f in directory.rglob("*") if f.suffix.lower() in SUPPORTED_EXTENSIONS]

    if not files:
        print(f"No supported media files found in {directory}")
        sys.exit(1)

    print(f"Found {len(files)} media files to process")
    print("-" * 50)

    # Process files in parallel
    results = []
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(process_file, f): f for f in files}

        for i, future in enumerate(as_completed(futures), 1):
            file_path = futures[future]
            result = future.result()
            results.append(result)

            status = "✓" if result["success"] else "✗"
            print(f"[{i}/{len(files)}] {status} {file_path.name}")

    # Find similar content
    print("\n" + "-" * 50)
    print("Finding similar content...")
    similar = find_similar_content(results, args.similarity_threshold)

    if similar:
        print(f"\nFound {len(similar)} similar pairs:")
        for pair in similar[:10]:
            print(f"  {pair['file1']} ↔ {pair['file2']}: {pair['similarity']:.1%}")
    else:
        print("No similar content found above threshold")

    # Generate report
    report = {
        "total_files": len(files),
        "successful": sum(1 for r in results if r["success"]),
        "failed": sum(1 for r in results if not r["success"]),
        "similar_pairs": similar,
        "files": results,
    }

    # Save or print report
    if args.output:
        with open(args.output, 'w') as f:
            json.dump(report, f, indent=2)
        print(f"\nReport saved to: {args.output}")
    else:
        print(f"\nSummary:")
        print(f"  Processed: {report['successful']}/{report['total_files']} files")
        print(f"  Failed:    {report['failed']} files")
        print(f"  Similar:   {len(similar)} pairs found")


if __name__ == "__main__":
    main()
