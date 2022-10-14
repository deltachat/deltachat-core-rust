ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@mailo.com header.s=mailo header.b="PoGUlxd/";
       spf=pass (google.com: domain of alice@mailo.com designates 213.182.54.11 as permitted sender) smtp.mailfrom=alice@mailo.com;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=mailo.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@mailo.com header.s=mailo header.b="PoGUlxd/";
       spf=pass (google.com: domain of alice@mailo.com designates 213.182.54.11 as permitted sender) smtp.mailfrom=alice@mailo.com;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=mailo.com
From: <alice@mailo.com>
To: <alice@gmail.com>
