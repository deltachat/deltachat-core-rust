ARC-Authentication-Results: i=2; mx.google.com;
       dkim=pass header.i=@outlook.com header.s=selector1 header.b=CHJ1fVli;
       arc=pass (i=1);
       spf=pass (google.com: domain of alice@outlook.com designates 40.92.66.108 as permitted sender) smtp.mailfrom=alice@outlook.com;
       dmarc=pass (p=NONE sp=QUARANTINE dis=NONE) header.from=outlook.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@outlook.com header.s=selector1 header.b=CHJ1fVli;
       arc=pass (i=1);
       spf=pass (google.com: domain of alice@outlook.com designates 40.92.66.108 as permitted sender) smtp.mailfrom=alice@outlook.com;
       dmarc=pass (p=NONE sp=QUARANTINE dis=NONE) header.from=outlook.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@gmail.com>
