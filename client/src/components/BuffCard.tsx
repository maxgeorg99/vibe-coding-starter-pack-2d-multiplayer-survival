import { useState } from 'react';

// Individual Buff Card Component
function BuffCard({ buff, isSelected, onSelect }) {
  return (
    <div 
      className={`relative w-64 cursor-pointer transform transition-all duration-300 ${
        isSelected ? 'scale-105 shadow-lg' : 'hover:scale-105 hover:shadow-lg'
      }`}
      onClick={() => onSelect(buff.id)}
    >
      {/* Card Frame */}
      <div className="relative">
        {/* Background */}
        <div 
          className="h-96 rounded-lg overflow-hidden transition-all duration-300 hover:brightness-110" 
          style={{ background: `linear-gradient(to bottom, ${buff.color || '#5252e5'}, #2a2a7a)` }}
        >
          
          {/* Gold corners */}
          <div className="absolute top-0 left-0 w-12 h-12 border-t-8 border-l-8 border-yellow-600 rounded-tl-lg transition-colors duration-300 hover:border-yellow-500"></div>
          <div className="absolute top-0 right-0 w-12 h-12 border-t-8 border-r-8 border-yellow-600 rounded-tr-lg transition-colors duration-300 hover:border-yellow-500"></div>
          <div className="absolute bottom-0 left-0 w-12 h-12 border-b-8 border-l-8 border-yellow-600 rounded-bl-lg transition-colors duration-300 hover:border-yellow-500"></div>
          <div className="absolute bottom-0 right-0 w-12 h-12 border-b-8 border-r-8 border-yellow-600 rounded-br-lg transition-colors duration-300 hover:border-yellow-500"></div>
          
          {/* Content */}
          <div className="flex flex-col items-center p-4 h-full">
            {/* Icon Circle with hover effect */}
            <div className="w-32 h-32 rounded-full bg-blue-900 border-4 border-yellow-600 flex items-center justify-center mb-6 mt-8 transition-all duration-300 hover:border-yellow-400 hover:shadow-md group">
              <span className="text-5xl transition-transform duration-300 group-hover:scale-110">{buff.icon}</span>
            </div>
            
            {/* Name */}
            <h2 className="text-xl font-bold text-white mb-4 transition-all duration-300 hover:text-yellow-100 hover:scale-105">{buff.name}</h2>
            
            {/* Divider with glow effect on hover */}
            <div className="w-3/4 h-px bg-gray-400 mb-4 transition-all duration-300 hover:bg-yellow-400 hover:h-0.5"></div>
            
            {/* Effect Text */}
            <p className="text-white text-center px-4 transition-colors duration-300 hover:text-yellow-100">{buff.effect}</p>
          </div>
        </div>
      </div>
      
      {/* Selection Indicator with animation */}
      {isSelected && (
        <div className="absolute inset-0 border-4 border-yellow-400 rounded-lg pointer-events-none animate-pulse"></div>
      )}
    </div>
  );
}