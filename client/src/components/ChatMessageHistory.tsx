import React, { useEffect, useRef, RefObject } from 'react';
import { Message as SpacetimeDBMessage, Player as SpacetimeDBPlayer } from '../generated'; // Assuming Message and Player types are generated
import styles from './Chat.module.css';

interface ChatMessageHistoryProps {
  messages: Map<string, SpacetimeDBMessage>; // Pass the messages map
  players: Map<string, SpacetimeDBPlayer>; // Pass players map to look up names
  messageEndRef: RefObject<HTMLDivElement>; // Add the ref parameter
}

const ChatMessageHistory: React.FC<ChatMessageHistoryProps> = ({ messages, players, messageEndRef }) => {
  const historyRef = useRef<HTMLDivElement>(null);

  // Sort messages by timestamp (assuming timestamp field exists and is comparable)
  const sortedMessages = Array.from(messages.values()).sort((a, b) => {
    // Assuming 'sent' is a SpacetimeDB Timestamp object
    const timeA = a.sent?.microsSinceUnixEpoch ?? 0n;
    const timeB = b.sent?.microsSinceUnixEpoch ?? 0n;
    if (timeA < timeB) return -1;
    if (timeA > timeB) return 1;
    return 0;
  });

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (historyRef.current) {
      historyRef.current.scrollTop = historyRef.current.scrollHeight;
    }
  }, [messages]); // Re-run effect when messages map changes

  const getPlayerName = (identityHex: string): string => {
    const player = players.get(identityHex);
    return player?.username ?? identityHex.substring(0, 8); // Fallback to short ID
  };

  return (
    <div ref={historyRef} className={styles.messageHistory}>
      {sortedMessages.map(msg => {
        const senderName = getPlayerName(msg.sender.toHexString());
        return (
          <div key={msg.id.toString()} className={styles.message}>
            <span className={styles.senderName}>{senderName}:</span>
            <span className={styles.messageText}>{msg.text}</span>
          </div>
        );
      })}
      <div ref={messageEndRef} />
    </div>
  );
};

export default ChatMessageHistory; 