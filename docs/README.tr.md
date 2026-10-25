# VidExtract

**Videoyu indir. Konuşmaların içinde ara. İstediğin ana git.**

VidExtract; video ve ses indirme, cihazda konuşmadan metne dönüştürme, zaman damgalı arama, çeviri ve MP4 kesit alma özelliklerini birleştiren GPLv3 lisanslı masaüstü uygulamasıdır. Arayüz Tauri 2, React ve Rust ile geliştirilmiştir.

- [Windows ve Linux sürümleri](https://github.com/idginity124/VidExtract/releases)
- [Kurulum ve derleme](../BUILDING.md)
- [Gizlilik politikası](../PRIVACY.md)
- [Microsoft Store hazırlığı](MICROSOFT-STORE.md)

Konuşma dili otomatik algılanabilir. Tiny/Base/Small modelleri cihazda çalışır; uygun ekran kartında Vulkan hızlandırması kullanılır. GPU çalışmazsa işlemciye geçilir.

“Çevir” düğmesine bastığında hedef dili ve yöntemi seçersin. Çevrimiçi çeviri MyMemory kullanır; kart, hesap veya API anahtarı gerekmez. Günlük ücretsiz sınır anonim kullanımda 5.000 karakterdir; geçerli ve isteğe bağlı e-posta ile 50.000 olabilir. Metin bu sağlayıcıya gönderilir. Çevrimdışı kullanımda yalnızca gereken 65–286 MB boyutundaki dil paketleri indirilir; bazı çiftlerde iki paket gerekir. Çeviri metni cihazda kalır.

Türkçe, İngilizce, İspanyolca, Fransızca, Almanca, Portekizce, Rusça, Arapça, Hintçe ve Çince arayüzü vardır. Orijinal ve çeviri yan yana görünür; TXT/SRT/VTT kaydedilebilir. Otomatik sonuçları kontrol etmek gerekir.

Eski Python arayüzü kaldırılmıştır. Yeni masaüstü kaynakları GPLv3 ile paylaşılır. Üçüncü taraf araçların ve modellerin lisansları [ayrıca belirtilir](../THIRD_PARTY.md).
