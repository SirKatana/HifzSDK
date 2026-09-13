// hifzsdk.swift — wired up by HifzSDK (Moga Sabqi → Manzil). Keep me.
import Foundation

// MARK: - HifzSDK Swift bridge
// Loads data/quran.json (whole Quran + sabqi/manzil memorization plan) from the app bundle.

public enum HifzSDK {

    /// The whole Quran, keyed by chapter: { "1": [ { chapter, verse, text }, ... ] }.
    public struct Verse: Codable, Identifiable, Hashable {
        public let chapter: Int
        public let verse: Int
        public let text: String
        public var id: String { "\(chapter):\(verse)" }
    }

    public struct VerseRef: Codable, Hashable {
        public let chapter: Int
        public let verse: Int
    }

    /// One day of the memorization plan.
    public struct Step: Codable, Identifiable {
        public let day: Int
        public let sabqiNew: [VerseRef]
        public let sabqiReview: [VerseRef]
        public let manzilReview: [VerseRef]
        public var id: Int { day }
    }

    public struct Plan: Codable {
        public let method: String
        public let description: String
        public let versesPerDay: Int
        public let sabqiReviewDays: Int
        public let manzilReviewDays: Int
        public let totalDays: Int
        public let steps: [Step]
    }

    public struct Quran: Codable {
        public let chapters: [String: [Verse]]
        public let hifzPlan: Plan
    }

    public static var bundle = Bundle.main

    /// Reads the bundled quran.json. Throws if the file is missing from the target.
    public static func load(bundle: Bundle = HifzSDK.bundle) throws -> Quran {
        let data = try Data(contentsOf: bundle.url(forResource: "quran", withExtension: "json")!)
        return try JSONDecoder().decode(Quran.self, from: data)
    }

    /// Resolves a VerseRef into the full Arabic text from the loaded Quran.
    public static func text(for ref: VerseRef, quran: Quran) -> String? {
        quran.chapters[String(ref.chapter)]?.first(where: { $0.verse == ref.verse })?.text
    }

    /// Todays' routine for any given day (1-indexed).
    public static func routine(atDay day: Int, quran: Quran) -> Step? {
        quran.hifzPlan.steps.first(where: { $0.day == day })
    }

    /// Full Arabic text for every verse in a step (sabqi new / review / manzil).
    public static func text(for refs: [VerseRef], quran: Quran) -> [String] {
        refs.compactMap { text(for: $0, quran: quran) }
    }
}
