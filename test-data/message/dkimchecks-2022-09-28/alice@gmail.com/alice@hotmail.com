ARC-Authentication-Results: i=2; mx.google.com;
       dkim=pass header.i=@hotmail.com header.s=selector1 header.b=cXkaZaq1;
       arc=pass (i=1);
       spf=pass (google.com: domain of alice@hotmail.com designates 40.92.73.35 as permitted sender) smtp.mailfrom=alice@hotmail.com;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=hotmail.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@hotmail.com header.s=selector1 header.b=cXkaZaq1;
       arc=pass (i=1);
       spf=pass (google.com: domain of alice@hotmail.com designates 40.92.73.35 as permitted sender) smtp.mailfrom=alice@hotmail.com;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=hotmail.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@gmail.com>
