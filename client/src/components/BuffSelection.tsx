export default function BuffSelection({ buffs = [], onBuffSelect }) {
    const [selectedBuff, setSelectedBuff] = useState(null);
    
    const handleSelect = (buffId) => {
      setSelectedBuff(buffId);
      if (onBuffSelect) {
        onBuffSelect(buffId);
      }
    };
  
    return (
      <div className="flex flex-col items-center p-6 bg-gray-800 min-h-screen">
        <h1 className="text-2xl font-bold text-white mb-8 hover:text-yellow-300 transition-colors duration-300">Select A Buff</h1>
        
        <div className="flex gap-4 justify-center flex-wrap">
          {buffs.map(buff => (
            <BuffCard
              key={buff.id}
              buff={buff}
              isSelected={selectedBuff === buff.id}
              onSelect={handleSelect}
            />
          ))}
        </div>
      </div>
    );
  }