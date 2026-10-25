#include <ctranslate2/translator.h>
#include <sentencepiece_processor.h>
#include <nlohmann/json.hpp>
#include <filesystem>
#include <fstream>
#include <iostream>
using json=nlohmann::json;
int main(int argc,char** argv){
 try{
  if(argc!=5)throw std::runtime_error("Expected package, input, output, threads");
  std::filesystem::path root=std::filesystem::u8path(argv[1]);
  std::ifstream input(std::filesystem::u8path(argv[2]));json payload;input>>payload;
  const auto texts=payload.at("texts").get<std::vector<std::string>>();
  if(texts.size()>200000)throw std::runtime_error("Input too large");
  std::ifstream sp_file(root/"sentencepiece.model",std::ios::binary);std::string sp_data((std::istreambuf_iterator<char>(sp_file)),{});
  sentencepiece::SentencePieceProcessor tokenizer;auto status=tokenizer.LoadFromSerializedProto(sp_data);if(!status.ok())throw std::runtime_error("Tokenizer unavailable");
  ctranslate2::ReplicaPoolConfig config;config.num_threads_per_replica=std::clamp(std::stoi(argv[4]),1,8);
  ctranslate2::Translator translator((root/"model").u8string(),ctranslate2::Device::CPU,ctranslate2::ComputeType::INT8,{0},false,config);
  ctranslate2::TranslationOptions options;options.beam_size=2;options.length_penalty=0.2;options.replace_unknowns=true;options.max_input_length=1024;options.max_decoding_length=1024;
  json output=json::array();
  std::string prefix=payload.value("prefix",std::string());
  for(size_t start=0;start<texts.size();start+=4){
   size_t end=std::min(start+4,texts.size());std::vector<std::vector<std::string>> batch;
   for(size_t i=start;i<end;i++){std::vector<std::string> tokens;auto encoded=tokenizer.Encode(texts[i],&tokens);if(!encoded.ok()||tokens.size()>1024)throw std::runtime_error("Input exceeds context");batch.push_back(std::move(tokens));}
   const auto results=prefix.empty()?translator.translate_batch(batch,options):translator.translate_batch(batch,std::vector<std::vector<std::string>>(batch.size(),{prefix}),options);
   if(results.size()!=batch.size())throw std::runtime_error("Missing translation");
   for(const auto& result:results){if(result.output().size()>=1024)throw std::runtime_error("Output exceeded context");std::string text;auto decoded=tokenizer.Decode(result.output(),&text);if(!decoded.ok()||text.empty())throw std::runtime_error("Empty translation");if(!prefix.empty()&&text.rfind(prefix,0)==0)text.erase(0,prefix.size());
    // Argos detokenization also removes residual SentencePiece word separators.
    const std::string marker=u8"\u2581";size_t position=0;while((position=text.find(marker,position))!=std::string::npos){text.replace(position,marker.size()," ");position++;}std::replace(text.begin(),text.end(),'_',' ');const auto first=text.find_first_not_of(" ");text=first==std::string::npos?std::string():text.substr(first);output.push_back(text);}
   std::cout<<"progress = "<<(end*100/texts.size())<<std::endl;
  }
  std::ofstream file(std::filesystem::u8path(argv[3]));file<<output.dump();if(!file)throw std::runtime_error("Could not write translation");return 0;
 }catch(const std::exception& error){std::cerr<<error.what()<<std::endl;return 1;}
}
