import React, { useEffect, useState } from "react";

interface StoryTextboxProps {
  onComplete: () => void;
}

const storyKeys = [
  "story.intro.1",
  "story.intro.2",
  "story.intro.3",
  "story.intro.4",
  "story.intro.5",
  "story.intro.6",
  "story.intro.7"
];

export const StoryTextbox: React.FC<StoryTextboxProps> = ({ onComplete }) => {
  const [translations, setTranslations] = useState<Record<string, string>>({});
  const [currentIndex, setCurrentIndex] = useState(0);
  const [displayedText, setDisplayedText] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [typingInterval, setTypingInterval] = useState<NodeJS.Timeout | null>(null);

  // Load locale
  useEffect(() => {
    fetch("/locales/de.json")
      .then((res) => res.json())
      .then((data) => setTranslations(data));
  }, []);

  // Typing effect
  useEffect(() => {
    const key = storyKeys[currentIndex];
    const fullText = translations[key];
    if (!fullText) return;

    setDisplayedText("");
    setIsTyping(true);
    let i = 0;

    const interval = setInterval(() => {
      i++;
      setDisplayedText(fullText.slice(0, i));
      if (i >= fullText.length) {
        clearInterval(interval);
        setIsTyping(false);
      }
    }, 40);

    setTypingInterval(interval);
    return () => clearInterval(interval);
  }, [currentIndex, translations]);

  const handleNext = () => {
    const key = storyKeys[currentIndex];
    const fullText = translations[key];

    if (isTyping) {
      if (typingInterval) clearInterval(typingInterval);
      setDisplayedText(fullText);
      setIsTyping(false);
      return;
    }

    if (currentIndex < storyKeys.length - 1) {
      setCurrentIndex(currentIndex + 1);
    } else {
      // Story has ended, trigger the onComplete callback
      onComplete();
    }
  };

  if (Object.keys(translations).length === 0) {
    return null;
  }

  const isLastStory = currentIndex === storyKeys.length - 1;

  return (
    <div className="fixed bottom-8 left-1/2 transform -translate-x-1/2 w-[90%] max-w-xl p-4 bg-black text-green-400 border-4 border-green-500 rounded-xl font-mono shadow-lg whitespace-pre-wrap">
      <p className="text-lg min-h-[6rem]">{displayedText}</p>
      <div className="text-right mt-4">
        <button
          onClick={handleNext}
          className="bg-green-600 hover:bg-green-700 text-white px-4 py-1 rounded shadow"
        >
          {isTyping ? "Überspringen" : isLastStory ? "Ende" : "Weiter"}
        </button>
      </div>
    </div>
  );
};

export default StoryTextbox;