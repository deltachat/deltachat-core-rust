ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@yandex.ru header.s=mail header.b="k4k4P0Z/";
       spf=pass (google.com: domain of alice@yandex.ru designates 77.88.28.108 as permitted sender) smtp.mailfrom=alice@yandex.ru;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=yandex.ru
Authentication-Results: mx.google.com;
       dkim=pass header.i=@yandex.ru header.s=mail header.b="k4k4P0Z/";
       spf=pass (google.com: domain of alice@yandex.ru designates 77.88.28.108 as permitted sender) smtp.mailfrom=alice@yandex.ru;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=yandex.ru
Authentication-Results: iva4-143b1447cf50.qloud-c.yandex.net; dkim=pass header.i=@yandex.ru
From: <alice@yandex.ru>
To: <alice@gmail.com>
